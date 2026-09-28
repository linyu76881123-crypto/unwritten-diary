//! 「不写日记」设备内本地核心。
//!
//! B1a 这一片只做资料库、迁移、幂等写入与记录路径（对应契约第 2.1、2.2 节与
//! 第 4.1 节的 captures / sources 部分）。原件文件库、录音、队列、搜索都还没有。
//!
//! 三条硬性约束贯穿本 crate：
//! 1. **不丢已确认的记录**：写入落在同一个事务里，事务提交后才返回 durable。
//! 2. **幂等**：可重试写入带 operationId，重复提交返回原结果；
//!    同一个 operationId 换了内容报 `idempotency_conflict`。
//! 3. **乐观锁**：带 expectedRevision 的写入不匹配就报 `revision_conflict`，
//!    绝不静默覆盖用户内容。

mod error;
mod model;
mod schema;
mod support;

pub use error::{CoreError, ErrorCode, Result};
pub use model::{
    AuthorType, Capture, CapturePage, CaptureState, CommitResult, DomainEvent, DraftSaveResult,
    EventType, ProcessingSummary, SourceItem, SourceRevision,
};
pub use schema::SCHEMA_VERSION;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction};

/// 创建草稿的入参。
#[derive(Debug, Clone)]
pub struct CreateDraftInput<'a> {
    /// 事件时间；为空时用当前时间。
    pub occurred_at: Option<DateTime<Utc>>,
    /// 设备当前的 IANA 时区名，例如 `Asia/Shanghai`。
    pub time_zone: &'a str,
    /// 事件发生时该时区的偏移分钟数。
    pub utc_offset_minutes: i32,
    pub operation_id: &'a str,
}

/// 资料库句柄。
pub struct Core {
    conn: Connection,
}

/// 一行的原始形态：先取出来，再按业务语义解析，避免把解析错误塞进 SQL 层。
type RawCapture = (String, i64, String, String, String, String, String, i64, String, String);

const CAPTURE_COLUMNS: &str = "id, revision, state, occurred_at, created_at, updated_at, \
                                time_zone, utc_offset_minutes, day_key, draft_text";

impl Core {
    /// 打开（或创建）指定路径的资料库。
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self> {
        Self::prepare(Connection::open(path)?)
    }

    /// 内存库，只用于测试。
    pub fn open_in_memory() -> Result<Self> {
        Self::prepare(Connection::open_in_memory()?)
    }

    fn prepare(mut conn: Connection) -> Result<Self> {
        // WAL 提高并发读写表现；synchronous=FULL 保证「提交了就是落盘了」，
        // 只用 NORMAL 会让进程崩溃时丢掉最后几个事务，与 durable 承诺不符。
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "FULL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        schema::migrate(&mut conn)?;
        Ok(Self { conn })
    }

    pub fn schema_version(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))?)
    }

    /// 仅供测试：把资料库可用的页数压小，用来触发 SQLite 的 `SQLITE_FULL`。
    ///
    /// 没有 root 权限时无法造一个真的满盘环境，这里用页数上限走同一条错误路径。
    #[doc(hidden)]
    pub fn set_page_limit_for_test(&self, pages: i64) -> Result<()> {
        self.conn.pragma_update(None, "max_page_count", pages)?;
        Ok(())
    }

    // ------------------------------------------------------------ 记录路径

    /// 创建草稿，契约第 4.1 节 `captures.createDraft`。
    pub fn create_draft(&mut self, input: CreateDraftInput<'_>) -> Result<Capture> {
        let occurred_hint = input.occurred_at.map(support::to_iso).unwrap_or_default();
        let fingerprint = support::fingerprint(&[
            "create_draft",
            &occurred_hint,
            input.time_zone,
            &input.utc_offset_minutes.to_string(),
        ]);
        if let Some(json) = self.receipt("create_draft", &fingerprint, input.operation_id)? {
            return Ok(serde_json::from_str(&json)?);
        }

        let now = Utc::now();
        let occurred_at = input.occurred_at.unwrap_or(now);
        let capture = Capture {
            id: support::new_id("cap"),
            revision: 1,
            state: CaptureState::Draft,
            occurred_at,
            created_at: now,
            updated_at: now,
            time_zone: input.time_zone.to_owned(),
            utc_offset_minutes: input.utc_offset_minutes,
            day_key: support::day_key(occurred_at, input.utc_offset_minutes),
            ordered_source_ids: Vec::new(),
            draft_text: String::new(),
            processing_summary: ProcessingSummary::default(),
        };

        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO captures (id, revision, state, occurred_at, created_at, updated_at, \
             time_zone, utc_offset_minutes, day_key, draft_text) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                capture.id,
                capture.revision,
                capture.state.wire(),
                support::to_iso(capture.occurred_at),
                support::to_iso(capture.created_at),
                support::to_iso(capture.updated_at),
                capture.time_zone,
                capture.utc_offset_minutes,
                capture.day_key,
                capture.draft_text,
            ],
        )?;
        store_receipt(
            &tx,
            input.operation_id,
            "create_draft",
            &fingerprint,
            &serde_json::to_string(&capture)?,
        )?;
        insert_event(&tx, EventType::CaptureChanged, &capture.id, capture.revision)?;
        tx.commit()?;
        Ok(capture)
    }

    /// 保存草稿正文。频繁保存只改草稿，不制造永久版本，契约第 2.1 节。
    pub fn save_draft(
        &mut self,
        capture_id: &str,
        text: &str,
        expected_revision: i64,
        operation_id: &str,
    ) -> Result<DraftSaveResult> {
        let fingerprint = support::fingerprint(&[
            "save_draft",
            capture_id,
            &expected_revision.to_string(),
            text,
        ]);
        if let Some(json) = self.receipt("save_draft", &fingerprint, operation_id)? {
            return Ok(serde_json::from_str(&json)?);
        }

        let capture = load_capture(&self.conn, capture_id)?;
        ensure_revision(&capture, expected_revision)?;
        if capture.state != CaptureState::Draft {
            return Err(CoreError::InvalidState {
                entity: "记录",
                id: capture.id.clone(),
                state: capture.state.wire().to_owned(),
            });
        }

        let now = Utc::now();
        let new_revision = capture.revision + 1;
        let result = DraftSaveResult {
            revision: new_revision,
            // 事务提交后才返回 true：界面据此显示「已保存」。
            durable: true,
            saved_at: now,
        };

        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE captures SET revision = ?1, draft_text = ?2, updated_at = ?3 WHERE id = ?4",
            params![new_revision, text, support::to_iso(now), capture.id],
        )?;
        store_receipt(
            &tx,
            operation_id,
            "save_draft",
            &fingerprint,
            &serde_json::to_string(&result)?,
        )?;
        insert_event(&tx, EventType::CaptureChanged, &capture.id, new_revision)?;
        tx.commit()?;
        Ok(result)
    }

    /// 提交记录：创建原始文字版本，契约第 4.1 节 `captures.commit`。
    pub fn commit(
        &mut self,
        capture_id: &str,
        expected_revision: i64,
        operation_id: &str,
    ) -> Result<CommitResult> {
        let fingerprint = support::fingerprint(&[
            "commit",
            capture_id,
            &expected_revision.to_string(),
        ]);
        if let Some(json) = self.receipt("commit", &fingerprint, operation_id)? {
            return Ok(serde_json::from_str(&json)?);
        }

        let capture = load_capture(&self.conn, capture_id)?;
        ensure_revision(&capture, expected_revision)?;
        if capture.state != CaptureState::Draft {
            return Err(CoreError::InvalidState {
                entity: "记录",
                id: capture.id.clone(),
                state: capture.state.wire().to_owned(),
            });
        }

        let now = Utc::now();
        let new_revision = capture.revision + 1;
        let mut ordered = capture.ordered_source_ids.clone();
        let mut original_text_revision = None;

        let tx = self.conn.transaction()?;
        // 空白草稿不制造空的原始文字版本；有内容才建 source + revision。
        if !capture.draft_text.is_empty() {
            let source_id = support::new_id("src");
            let revision_id = support::new_id("rev");
            let position = ordered.len() as i64;
            tx.execute(
                "INSERT INTO source_items (source_id, capture_id, kind, position, \
                 current_revision_id) VALUES (?1, ?2, 'text', ?3, ?4)",
                params![source_id, capture.id, position, revision_id],
            )?;
            tx.execute(
                "INSERT INTO source_revisions (revision_id, source_id, parent_revision_id, text, \
                 asset_id, author_type, occurred_at) VALUES (?1, ?2, NULL, ?3, NULL, ?4, ?5)",
                params![
                    revision_id,
                    source_id,
                    capture.draft_text,
                    AuthorType::User.wire(),
                    support::to_iso(now),
                ],
            )?;
            ordered.push(source_id.clone());
            original_text_revision = Some(SourceRevision {
                revision_id,
                source_id,
                parent_revision_id: None,
                text: Some(capture.draft_text.clone()),
                asset_id: None,
                author_type: AuthorType::User,
                occurred_at: now,
            });
        }

        let mut committed = capture.clone();
        committed.revision = new_revision;
        committed.state = CaptureState::Committed;
        committed.updated_at = now;
        committed.ordered_source_ids = ordered;

        tx.execute(
            "UPDATE captures SET revision = ?1, state = 'committed', updated_at = ?2 WHERE id = ?3",
            params![new_revision, support::to_iso(now), capture.id],
        )?;

        let result = CommitResult {
            capture: committed,
            original_text_revision,
        };
        store_receipt(
            &tx,
            operation_id,
            "commit",
            &fingerprint,
            &serde_json::to_string(&result)?,
        )?;
        insert_event(&tx, EventType::CaptureChanged, &capture.id, new_revision)?;
        tx.commit()?;
        Ok(result)
    }

    /// 修改原始文字：创建新的 SourceRevision，旧版本保留。
    pub fn revise_text(
        &mut self,
        source_id: &str,
        text: &str,
        expected_revision: i64,
        operation_id: &str,
    ) -> Result<SourceRevision> {
        let fingerprint = support::fingerprint(&[
            "revise_text",
            source_id,
            &expected_revision.to_string(),
            text,
        ]);
        if let Some(json) = self.receipt("revise_text", &fingerprint, operation_id)? {
            return Ok(serde_json::from_str(&json)?);
        }

        let (capture_id, parent_revision_id) = load_source_item(&self.conn, source_id)?;
        let capture = load_capture(&self.conn, &capture_id)?;
        // 乐观锁以记录的 revision 为准：记录是聚合根，一次修订同时推进它。
        ensure_revision(&capture, expected_revision)?;
        if capture.state == CaptureState::Draft {
            return Err(CoreError::InvalidState {
                entity: "记录",
                id: capture.id.clone(),
                state: capture.state.wire().to_owned(),
            });
        }

        let now = Utc::now();
        let revision = SourceRevision {
            revision_id: support::new_id("rev"),
            source_id: source_id.to_owned(),
            parent_revision_id,
            text: Some(text.to_owned()),
            asset_id: None,
            author_type: AuthorType::User,
            occurred_at: now,
        };
        let new_revision = capture.revision + 1;

        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO source_revisions (revision_id, source_id, parent_revision_id, text, \
             asset_id, author_type, occurred_at) VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6)",
            params![
                revision.revision_id,
                revision.source_id,
                revision.parent_revision_id,
                revision.text,
                revision.author_type.wire(),
                support::to_iso(now),
            ],
        )?;
        tx.execute(
            "UPDATE source_items SET current_revision_id = ?1 WHERE source_id = ?2",
            params![revision.revision_id, source_id],
        )?;
        tx.execute(
            "UPDATE captures SET revision = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_revision, support::to_iso(now), capture.id],
        )?;
        store_receipt(
            &tx,
            operation_id,
            "revise_text",
            &fingerprint,
            &serde_json::to_string(&revision)?,
        )?;
        insert_event(&tx, EventType::CaptureChanged, &capture.id, new_revision)?;
        tx.commit()?;
        Ok(revision)
    }

    // ------------------------------------------------------------ 读取

    pub fn get_capture(&self, capture_id: &str) -> Result<Capture> {
        load_capture(&self.conn, capture_id)
    }

    /// 分页读取。按 `occurred_at DESC, id DESC` 确定性排序，cursor 对调用方不透明。
    pub fn list_captures(
        &self,
        day_key: Option<&str>,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<CapturePage> {
        let (cursor_at, cursor_id) = match cursor {
            Some(raw) => {
                let (at, id) = support::split_cursor(raw)?;
                (Some(support::to_iso(at)), Some(id))
            }
            None => (None, None),
        };
        let page_size = limit.clamp(1, 100);

        let sql = format!(
            "SELECT {CAPTURE_COLUMNS} FROM captures \
             WHERE (?1 IS NULL OR day_key = ?1) \
               AND (?2 IS NULL OR occurred_at < ?2 OR (occurred_at = ?2 AND id < ?3)) \
             ORDER BY occurred_at DESC, id DESC LIMIT ?4"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map(
                params![day_key, cursor_at, cursor_id, (page_size + 1) as i64],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, String>(8)?,
                        row.get::<_, String>(9)?,
                    ))
                },
            )?
            .collect::<rusqlite::Result<Vec<RawCapture>>>()?;

        let mut captures = Vec::with_capacity(rows.len().min(page_size));
        for raw in rows {
            captures.push(capture_from_raw(&self.conn, raw)?);
        }
        // 多取一条用来判断还有没有下一页。
        let next_cursor = if captures.len() > page_size {
            captures.truncate(page_size);
            captures
                .last()
                .map(|capture| support::join_cursor(capture.occurred_at, &capture.id))
        } else {
            None
        };

        Ok(CapturePage {
            captures,
            next_cursor,
        })
    }

    /// 从序号之后读取事件，契约第 6 节。
    pub fn events_since(&self, from_sequence: i64) -> Result<Vec<DomainEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT sequence, event_id, type, entity_id, revision, emitted_at \
             FROM domain_events WHERE sequence > ?1 ORDER BY sequence",
        )?;
        let rows = stmt
            .query_map(params![from_sequence], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut events = Vec::with_capacity(rows.len());
        for (sequence, event_id, event_type, entity_id, revision, emitted_at) in rows {
            events.push(DomainEvent {
                event_id,
                sequence,
                event_type: EventType::from_wire(&event_type).ok_or_else(|| {
                    CoreError::CorruptedData {
                        message: format!("未知事件类型：{event_type}"),
                    }
                })?,
                entity_id,
                revision,
                emitted_at: support::parse_iso(&emitted_at)?,
            });
        }
        Ok(events)
    }

    // ------------------------------------------------------------ 幂等回执

    fn receipt(
        &self,
        kind: &str,
        fingerprint: &str,
        operation_id: &str,
    ) -> Result<Option<String>> {
        let row: Option<(String, String, String)> = self
            .conn
            .query_row(
                "SELECT kind, request_fingerprint, result_json FROM operation_receipts \
                 WHERE operation_id = ?1",
                params![operation_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;

        match row {
            None => Ok(None),
            Some((stored_kind, stored_fingerprint, result_json)) => {
                if stored_kind != kind || stored_fingerprint != fingerprint {
                    return Err(CoreError::IdempotencyConflict {
                        operation_id: operation_id.to_owned(),
                    });
                }
                Ok(Some(result_json))
            }
        }
    }
}

// ------------------------------------------------------------------ 内部工具

fn ensure_revision(capture: &Capture, expected_revision: i64) -> Result<()> {
    if capture.revision != expected_revision {
        return Err(CoreError::RevisionConflict {
            expected: expected_revision,
            actual: capture.revision,
        });
    }
    Ok(())
}

fn store_receipt(
    tx: &Transaction<'_>,
    operation_id: &str,
    kind: &str,
    fingerprint: &str,
    result_json: &str,
) -> Result<()> {
    tx.execute(
        "INSERT INTO operation_receipts (operation_id, kind, request_fingerprint, result_json, \
         created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            operation_id,
            kind,
            fingerprint,
            result_json,
            support::to_iso(Utc::now())
        ],
    )?;
    Ok(())
}

fn insert_event(
    tx: &Transaction<'_>,
    event_type: EventType,
    entity_id: &str,
    revision: i64,
) -> Result<()> {
    tx.execute(
        "INSERT INTO domain_events (event_id, type, entity_id, revision, emitted_at) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            support::new_id("evt"),
            event_type.wire(),
            entity_id,
            revision,
            support::to_iso(Utc::now())
        ],
    )?;
    Ok(())
}

fn load_source_item(conn: &Connection, source_id: &str) -> Result<(String, Option<String>)> {
    conn.query_row(
        "SELECT capture_id, current_revision_id FROM source_items WHERE source_id = ?1",
        params![source_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
    )
    .optional()?
    .ok_or_else(|| CoreError::NotFound {
        entity: "来源",
        id: source_id.to_owned(),
    })
}

fn load_capture(conn: &Connection, capture_id: &str) -> Result<Capture> {
    let sql = format!("SELECT {CAPTURE_COLUMNS} FROM captures WHERE id = ?1");
    let raw: Option<RawCapture> = conn
        .query_row(&sql, params![capture_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get(8)?,
                row.get(9)?,
            ))
        })
        .optional()?;

    match raw {
        Some(row) => capture_from_raw(conn, row),
        None => Err(CoreError::NotFound {
            entity: "记录",
            id: capture_id.to_owned(),
        }),
    }
}

fn capture_from_raw(conn: &Connection, raw: RawCapture) -> Result<Capture> {
    let (id, revision, state, occurred_at, created_at, updated_at, time_zone, offset, day_key, draft_text) = raw;
    let mut stmt =
        conn.prepare("SELECT source_id FROM source_items WHERE capture_id = ?1 ORDER BY position")?;
    let ordered_source_ids = stmt
        .query_map(params![id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<String>>>()?;

    Ok(Capture {
        id,
        revision,
        state: CaptureState::from_wire(&state).ok_or_else(|| CoreError::CorruptedData {
            message: format!("未知记录状态：{state}"),
        })?,
        occurred_at: support::parse_iso(&occurred_at)?,
        created_at: support::parse_iso(&created_at)?,
        updated_at: support::parse_iso(&updated_at)?,
        time_zone,
        utc_offset_minutes: i32::try_from(offset).map_err(|_| CoreError::CorruptedData {
            message: format!("时区偏移超出范围：{offset}"),
        })?,
        day_key,
        ordered_source_ids,
        draft_text,
        // B1b/B2 接上提取与索引后才会出现非零值。
        processing_summary: ProcessingSummary::default(),
    })
}