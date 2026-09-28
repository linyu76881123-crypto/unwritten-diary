//! schema 版本与迁移。
//!
//! 契约第 1.3 节要求 CoreInfo 报告 dataSchemaVersion；任务书 3.4 节要求迁移
//! 带明确版本、失败时保留旧资料。这里用 `PRAGMA user_version` 记录版本，
//! 迁移本身放在事务里，失败就整体回退。

use rusqlite::{params, Connection};

/// 本构建支持的 schema 版本。
pub const SCHEMA_VERSION: i64 = 1;

/// 迁移到最新版本。已经是最新则什么都不做。
pub fn migrate(conn: &mut Connection) -> rusqlite::Result<()> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    if current > SCHEMA_VERSION {
        // 旧构建不能打开新库：宁可不打开，也不要用错误的假设写坏资料。
        return Err(rusqlite::Error::InvalidParameterName(format!(
            "资料库的 schemaVersion 是 {current}，高于本构建支持的 {SCHEMA_VERSION}"
        )));
    }
    if current == SCHEMA_VERSION {
        return Ok(());
    }

    let tx = conn.transaction()?;
    if current < 1 {
        tx.execute_batch(V1)?;
    }
    tx.execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION};"))?;
    tx.execute(
        "INSERT INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
        params![SCHEMA_VERSION, crate::support::to_iso(chrono::Utc::now())],
    )?;
    tx.commit()
}

/// v1：记录、原始内容、幂等回执、域事件。
///
/// 索引与派生内容（extracted_contents、searchable_chunks）不在 v1 里，
/// 它们随 B1b/B2 一起进来。
const V1: &str = r#"
CREATE TABLE schema_migrations (
    version    INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);

CREATE TABLE captures (
    id                 TEXT PRIMARY KEY,
    revision           INTEGER NOT NULL,
    state              TEXT NOT NULL,
    occurred_at        TEXT NOT NULL,
    created_at         TEXT NOT NULL,
    updated_at         TEXT NOT NULL,
    time_zone          TEXT NOT NULL,
    utc_offset_minutes INTEGER NOT NULL,
    day_key            TEXT NOT NULL,
    draft_text         TEXT NOT NULL DEFAULT ''
);
CREATE INDEX idx_captures_day ON captures(day_key, occurred_at DESC, id DESC);

CREATE TABLE source_items (
    source_id           TEXT PRIMARY KEY,
    capture_id          TEXT NOT NULL REFERENCES captures(id) ON DELETE CASCADE,
    kind                TEXT NOT NULL,
    position            INTEGER NOT NULL,
    current_revision_id TEXT
);
CREATE INDEX idx_source_items_capture ON source_items(capture_id, position);

CREATE TABLE source_revisions (
    revision_id        TEXT PRIMARY KEY,
    source_id          TEXT NOT NULL REFERENCES source_items(source_id) ON DELETE CASCADE,
    parent_revision_id TEXT,
    text               TEXT,
    asset_id           TEXT,
    author_type        TEXT NOT NULL,
    occurred_at        TEXT NOT NULL
);
CREATE INDEX idx_source_revisions_source ON source_revisions(source_id, occurred_at);

CREATE TABLE operation_receipts (
    operation_id         TEXT PRIMARY KEY,
    kind                 TEXT NOT NULL,
    request_fingerprint  TEXT NOT NULL,
    result_json          TEXT NOT NULL,
    created_at           TEXT NOT NULL
);

CREATE TABLE domain_events (
    sequence   INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id   TEXT NOT NULL UNIQUE,
    type       TEXT NOT NULL,
    entity_id  TEXT NOT NULL,
    revision   INTEGER NOT NULL,
    emitted_at TEXT NOT NULL
);
CREATE INDEX idx_domain_events_entity ON domain_events(entity_id, sequence);
"#;