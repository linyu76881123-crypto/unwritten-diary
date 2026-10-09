//! 检索会话：契约第 2.6 节与第 4.4 节的 `search.*`。
//!
//! 三条规则决定了这里的形状：
//!
//! 1. **旧查询的结果不能覆盖新查询**。核心按 `sessionId` + `queryRevision` 双校验；
//!    会话不存在、或索引在会话期间变了，一律报 `search_expired` 让前端重新发起，
//!    而不是把过期结果糊到新界面上。
//! 2. **翻页绑定原会话**。游标里带着会话号，拿别的会话的游标来翻报 `cursor_expired`。
//! 3. **取消只停止后续处理，不删任何原件**。取消后 `phase` 是 `cancelled`，
//!    再翻页会被拒（`invalid_state`），但已经返回的结果仍然可读。
//!
//! 会话状态放在内存里：它记录的是「这一次查询翻到哪、快照还有效吗」，不是需要
//! 长期保存的业务数据。重开资料库后旧 `sessionId` 一律 `search_expired`。

use std::collections::HashMap;

use crate::error::{CoreError, Result};
use crate::model::{
    Coverage, MatchedBy, SearchFilters, SearchHit, SearchMode, SearchPhase, SearchRequest,
    SearchSnapshot, SourceKind, TextRange,
};
use crate::search;
use crate::support;
use crate::Core;

/// 一页最多这么多条。契约的默认每页 20；上限挡住「一次要一万条」这种请求。
const MAX_PAGE_SIZE: i64 = 100;

/// 摘录窗口：命中位置前后各留这么多字符。
const SNIPPET_PADDING: usize = 24;

/// 索引指纹：会话期间变了就说明快照过期。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IndexFingerprint {
    docs: i64,
    doc_id_sum: i64,
}

/// 一个检索会话的内存状态。
pub(crate) struct SearchSessionState {
    session_id: String,
    query_revision: i64,
    request: SearchRequest,
    /// 命中的 doc_id，已按排序规则排好。只存 id：整段正文按页现取，别把几万条
    /// 正文常驻内存。
    ordered: Vec<i64>,
    /// 上一页返回的 doc_id，`search.snapshot` 要能原样再给一次。
    last_page: Vec<i64>,
    /// 下一页的游标；没有下一页时为 None。
    next_cursor: Option<String>,
    fingerprint: IndexFingerprint,
    phase: SearchPhase,
    cancelled: bool,
    index_coverage: Coverage,
    warnings: Vec<String>,
}

pub(crate) fn start(
    core: &mut Core,
    request: SearchRequest,
    query_revision: i64,
) -> Result<SearchSnapshot> {
    let page_size = request.page_size.clamp(1, MAX_PAGE_SIZE);
    let needle = request.query.trim().to_lowercase();
    let mut warnings = Vec::new();

    if request.mode != SearchMode::Keyword {
        // 语义与混合还没接：如实降级到关键词，并说清楚，而不是假装按混合跑了。
        warnings.push(format!(
            "当前只实现了关键词检索，这次按关键词执行（请求的是 {}）",
            request.mode.wire()
        ));
    }
    if request.filters.include_old_diary_versions {
        warnings.push("日记历史版本还没有进索引，这个开关暂时没有效果".to_owned());
    }

    let ordered = if needle.is_empty() {
        warnings.push("查询是空的，没有可匹配的内容".to_owned());
        Vec::new()
    } else {
        search::ranked_matches(core, &needle, &request.filters)?
            .into_iter()
            .map(|row| row.doc_id)
            .collect()
    };

    let index_coverage = index_coverage(core)?;
    if index_coverage != Coverage::Complete {
        warnings.push("索引还没有覆盖全部内容，结果可能不完整".to_owned());
    }

    let session_id = support::new_id("search");
    let fingerprint = fingerprint(core)?;
    let (last_page, next_cursor) = page_window(&session_id, &ordered, 0, page_size);
    let phase = if next_cursor.is_none() {
        SearchPhase::Done
    } else {
        SearchPhase::KeywordReady
    };

    let state = SearchSessionState {
        session_id: session_id.clone(),
        query_revision,
        request: SearchRequest {
            query: request.query,
            mode: request.mode,
            filters: request.filters,
            page_size,
        },
        ordered,
        last_page,
        next_cursor,
        fingerprint,
        phase,
        cancelled: false,
        index_coverage,
        warnings,
    };
    let snapshot = materialize(core, &state)?;
    core.search_sessions_mut().insert(session_id, state);
    Ok(snapshot)
}

pub(crate) fn next_page(
    core: &mut Core,
    session_id: &str,
    cursor: Option<&str>,
) -> Result<SearchSnapshot> {
    // 先把游标解出来再借用会话：借错了顺序就编译不过，顺便逼着校验游标归属。
    let offset = match cursor {
        Some(value) => parse_cursor(value, session_id)?,
        None => None,
    };

    let mut state = core
        .search_sessions_mut()
        .remove(session_id)
        .ok_or_else(|| CoreError::SearchExpired {
            reason: format!("会话 {session_id} 不存在或已随重启丢失，请重新发起查询"),
        })?;

    let result = (|| -> Result<SearchSnapshot> {
        if state.cancelled {
            return Err(CoreError::InvalidState {
                entity: "检索会话",
                id: state.session_id.clone(),
                state: "cancelled".to_owned(),
            });
        }
        ensure_fresh(core, &state)?;

        let start = match offset {
            // 显式游标：必须指向这一页之后，否则就是拿旧游标重翻。
            Some(value) => {
                let expected = state.next_cursor.as_deref().map(parse_offset).transpose()?;
                if Some(value) != expected {
                    return Err(CoreError::CursorExpired {
                        reason: format!(
                            "游标与当前进度对不上（会话 {}，收到 {}）",
                            state.session_id, value
                        ),
                    });
                }
                value
            }
            None => state.next_cursor.as_deref().map(parse_offset).transpose()?.unwrap_or(0),
        };

        let (page, cursor) = page_window(
            &state.session_id,
            &state.ordered,
            start,
            state.request.page_size,
        );
        state.last_page = page;
        state.next_cursor = cursor;
        state.phase = if state.next_cursor.is_none() {
            SearchPhase::Done
        } else {
            SearchPhase::KeywordReady
        };
        materialize(core, &state)
    })();

    // 无论成功失败都要把会话放回去：失败（比如游标过期）不该把整个会话弄没。
    core.search_sessions_mut().insert(session_id.to_owned(), state);
    result
}

pub(crate) fn snapshot(core: &Core, session_id: &str) -> Result<SearchSnapshot> {
    let state = core
        .search_sessions()
        .get(session_id)
        .ok_or_else(|| CoreError::SearchExpired {
            reason: format!("会话 {session_id} 不存在或已随重启丢失，请重新发起查询"),
        })?;
    ensure_fresh(core, state)?;
    materialize(core, state)
}

pub(crate) fn cancel(core: &mut Core, session_id: &str) -> Result<SearchSnapshot> {
    let mut state = core
        .search_sessions_mut()
        .remove(session_id)
        .ok_or_else(|| CoreError::SearchExpired {
            reason: format!("会话 {session_id} 不存在或已随重启丢失，请重新发起查询"),
        })?;
    state.cancelled = true;
    state.phase = SearchPhase::Cancelled;
    state.next_cursor = None;
    let result = materialize(core, &state);
    // 取消后还留在表里：这样 `search.snapshot` 还能读到「已取消」这个状态，
    // 前端不至于因为会话突然消失而把它当成过期（那是两件事）。
    core.search_sessions_mut().insert(session_id.to_owned(), state);
    result
}

// ------------------------------------------------------------ 内部

fn ensure_fresh(core: &Core, state: &SearchSessionState) -> Result<()> {
    if fingerprint(core)? != state.fingerprint {
        return Err(CoreError::SearchExpired {
            reason: "索引在这次查询期间变了，快照已失效；请重新发起查询".to_owned(),
        });
    }
    Ok(())
}

fn fingerprint(core: &Core) -> Result<IndexFingerprint> {
    let (docs, doc_id_sum) = core.conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(doc_id), 0) FROM search_docs",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    Ok(IndexFingerprint { docs, doc_id_sum })
}

/// 索引覆盖：复用 `indexes.status` 的口径，但只关心「能不能搜」。
fn index_coverage(core: &Core) -> Result<Coverage> {
    Ok(crate::search::status(core, None)?.coverage)
}

/// 取一页与下一页游标。越界或空结果时下一页游标是 None。
fn page_window(
    session_id: &str,
    ordered: &[i64],
    start: usize,
    page_size: i64,
) -> (Vec<i64>, Option<String>) {
    if start >= ordered.len() {
        return (Vec::new(), None);
    }
    let end = (start + page_size as usize).min(ordered.len());
    let page = ordered[start..end].to_vec();
    let cursor = if end < ordered.len() {
        Some(format!("{session_id}:{}", end))
    } else {
        None
    };
    (page, cursor)
}

/// 游标是 `<sessionId>:<offset>`。会话号对不上就是 `cursor_expired`——
/// 契约要求翻页绑定原会话，不能拿别的会话的游标接着翻。
fn parse_cursor(value: &str, session_id: &str) -> Result<Option<usize>> {
    let (owner, offset) = value.rsplit_once(':').ok_or_else(|| CoreError::CursorExpired {
        reason: format!("游标格式不对：{value}"),
    })?;
    if owner != session_id {
        return Err(CoreError::CursorExpired {
            reason: format!("游标属于别的会话（{owner}），不能用来翻当前会话"),
        });
    }
    Ok(Some(offset.parse::<usize>().map_err(|_| CoreError::CursorExpired {
        reason: format!("游标里的位置不是数字：{value}"),
    })?))
}

fn parse_offset(cursor: &str) -> Result<usize> {
    cursor
        .rsplit_once(':')
        .and_then(|(_, offset)| offset.parse::<usize>().ok())
        .ok_or_else(|| CoreError::CursorExpired {
            reason: format!("游标格式不对：{cursor}"),
        })
}

/// 把这一页的 doc_id 变成命中对象。整段正文只在这一步取，按页取。
fn materialize(core: &Core, state: &SearchSessionState) -> Result<SearchSnapshot> {
    let needle = state.request.query.trim().to_lowercase();
    let rows = search::load_page(core, &state.last_page)?;
    let mut by_id: HashMap<i64, _> = rows.into_iter().map(|row| (row.doc_id, row)).collect();

    let mut results = Vec::with_capacity(state.last_page.len());
    for doc_id in &state.last_page {
        let Some(row) = by_id.remove(doc_id) else {
            continue;
        };
        let (snippet, highlights) = snippet_of(&row.text, &needle);
        results.push(SearchHit {
            hit_id: row.segment_id.clone(),
            group_id: row.source_id.clone(),
            source_kind: SourceKind::from_wire(&row.kind),
            matched_by: vec![MatchedBy::Keyword],
            coverage: row.coverage,
            source_id: Some(row.source_id),
            revision_id: Some(row.source_revision_id.clone()),
            day_key: row.day_key,
            title: row.asset_name,
            snippet,
            highlights,
            locator: Some(row.locator),
        });
    }

    Ok(SearchSnapshot {
        session_id: state.session_id.clone(),
        query_revision: state.query_revision,
        phase: state.phase,
        results,
        cursor: state.next_cursor.clone(),
        index_coverage: state.index_coverage,
        warnings: state.warnings.clone(),
    })
}

/// 命中附近的摘录与高亮。高亮下标**相对摘录**，不是相对整段正文。
fn snippet_of(text: &str, needle: &str) -> (Option<String>, Vec<TextRange>) {
    let characters: Vec<char> = text.chars().collect();
    let needle_length = needle.chars().count();
    let lowered = text.to_lowercase();
    let match_start = lowered
        .find(needle)
        .map(|byte_index| lowered[..byte_index].chars().count());
    let Some(start) = match_start else {
        return (None, Vec::new());
    };

    let window_start = start.saturating_sub(SNIPPET_PADDING);
    let window_end = (start + needle_length + SNIPPET_PADDING).min(characters.len());
    let mut snippet = String::new();
    let mut offset = 0_i64;
    if window_start > 0 {
        snippet.push('…');
        offset = 1;
    }
    snippet.extend(&characters[window_start..window_end]);
    if window_end < characters.len() {
        snippet.push('…');
    }

    let highlight_start = (start - window_start) as i64 + offset;
    (
        Some(snippet),
        vec![TextRange {
            start: highlight_start,
            end: highlight_start + needle_length as i64,
        }],
    )
}

/// 命中的排序键：日期从近到远，再按出现位置，最后按写入顺序。
///
/// 为什么不用「整词命中优先」：那需要在候选阶段为每一行查一次词表，代价随候选数
/// 线性增长；这一片先把可解释的排序做出来，等混合检索带分数进来再一起做。
pub(crate) fn sort_key(row: &search::RankedMatch) -> (i64, i64, i64) {
    // 没有 day_key 的排最后（用很小的负数代表「很旧」）。
    let day = row
        .day_key
        .as_deref()
        .and_then(|value| {
            // day_key 是 YYYY-MM-DD，直接当字符串比较即可，排序时转成可比数字。
            let mut parts = value.split('-');
            let year = parts.next()?.parse::<i64>().ok()?;
            let month = parts.next()?.parse::<i64>().ok()?;
            let day = parts.next()?.parse::<i64>().ok()?;
            Some(year * 10_000 + month * 100 + day)
        })
        .unwrap_or(i64::MIN);
    (-day, row.position, row.doc_id)
}

/// 会话表只给本模块用；放在 `Core` 上的小访问器避免把字段公开出去。
impl Core {
    pub(crate) fn search_sessions(&self) -> &HashMap<String, SearchSessionState> {
        &self.search_sessions
    }

    pub(crate) fn search_sessions_mut(&mut self) -> &mut HashMap<String, SearchSessionState> {
        &mut self.search_sessions
    }
}

/// 过滤条件原样带进 SQL；这里只做类型检查，不做语义解释。
pub(crate) fn filters_to_params(filters: &SearchFilters) -> Result<(Option<String>, Option<String>)> {
    let kinds = if filters.kinds.is_empty() {
        None
    } else {
        Some(serde_json::to_string(
            &filters
                .kinds
                .iter()
                .map(|kind| kind.wire())
                .collect::<Vec<_>>(),
        )?)
    };
    let scope = match &filters.source_scope {
        Some(values) if !values.is_empty() => Some(serde_json::to_string(values)?),
        _ => None,
    };
    Ok((kinds, scope))
}
