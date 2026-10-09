//! B3b 检索会话测试：分页、游标归属、快照失效、取消、过滤与摘录。
//!
//! 语料用真实路径造：建记录 → 导入文本 → 提取（同时入索引），再走 `search.*`。

use std::path::Path;

use chrono::{TimeZone, Utc};
use diary_core::{
    Core, Coverage, CreateDraftInput, ErrorCode, ImportManifest, ImportOrigin, ImportRequest,
    MatchedBy, SearchFilters, SearchMode, SearchPhase, SearchRequest, SourceKind,
};
use sha2::{Digest, Sha256};

fn sha_of(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn new_core(dir: &Path) -> Core {
    Core::open_in_memory_at(dir).unwrap()
}

/// 建一条指定日期的记录，导入文本并提取（提取会把正文写进索引）。
fn add_material(
    core: &mut Core,
    name: &str,
    text: &str,
    occurred: chrono::DateTime<Utc>,
    operation: &str,
) -> String {
    let bytes = text.as_bytes();
    let capture_id = core
        .create_draft(CreateDraftInput {
            occurred_at: Some(occurred),
            time_zone: "Asia/Shanghai",
            utc_offset_minutes: 480,
            operation_id: &format!("{operation}-create"),
        })
        .unwrap()
        .id;
    let ticket = core
        .prepare_import(ImportRequest {
            capture_id: &capture_id,
            display_name: name,
            mime_hint: Some("text/plain"),
            size_hint: Some(bytes.len() as i64),
            origin: ImportOrigin::Picker,
            operation_id: &format!("{operation}-prepare"),
        })
        .unwrap();
    std::fs::write(&ticket.staging_ticket, bytes).unwrap();
    core.finish_import(
        &ticket.import_id,
        &ticket.staging_ticket,
        ImportManifest {
            copied_bytes: bytes.len() as i64,
            sha256: sha_of(bytes),
            detected_mime: "text/plain".to_owned(),
            original_name: name.to_owned(),
        },
    )
    .unwrap();
    let source_id = core
        .get_capture(&capture_id)
        .unwrap()
        .ordered_source_ids
        .first()
        .cloned()
        .unwrap();
    core.extract_source(&source_id).unwrap();
    source_id
}

fn request(query: &str, page_size: i64) -> SearchRequest {
    SearchRequest {
        query: query.to_owned(),
        mode: SearchMode::Keyword,
        filters: SearchFilters::default(),
        page_size,
    }
}

/// 十段都含「妈妈」的文本，用空行分段。
fn ten_paragraphs() -> String {
    let mut text = String::new();
    for index in 0..10 {
        text.push_str(&format!("第{index}段，妈妈今天打电话来，聊了很久。"));
        text.push_str("\n\n");
    }
    text
}

#[test]
fn paging_is_stable_and_stops_at_the_end() {
    let dir = tempfile::tempdir().unwrap();
    let mut core = new_core(dir.path());
    add_material(
        &mut core,
        "日记.txt",
        &ten_paragraphs(),
        Utc.with_ymd_and_hms(2026, 9, 20, 4, 0, 0).unwrap(),
        "op1",
    );

    let first = core.start_search(request("妈妈", 3), 1).unwrap();
    assert_eq!(first.phase, SearchPhase::KeywordReady);
    assert_eq!(first.query_revision, 1);
    assert_eq!(first.results.len(), 3);
    assert!(first.cursor.is_some());
    assert!(first.warnings.is_empty(), "{:?}", first.warnings);
    assert_eq!(first.index_coverage, Coverage::Complete);

    let mut seen: Vec<String> = first.results.iter().map(|hit| hit.hit_id.clone()).collect();
    let mut cursor = first.cursor.clone();
    let mut pages = 1;
    while let Some(value) = cursor {
        let page = core.search_next_page(&first.session_id, Some(&value)).unwrap();
        pages += 1;
        seen.extend(page.results.iter().map(|hit| hit.hit_id.clone()));
        cursor = page.cursor.clone();
        if cursor.is_none() {
            assert_eq!(page.phase, SearchPhase::Done, "最后一页应当是 done");
        }
    }

    assert_eq!(pages, 4, "10 条按每页 3 条应当是 4 页");
    assert_eq!(seen.len(), 10, "翻完所有页应当不重不漏");
    let unique: std::collections::HashSet<_> = seen.iter().collect();
    assert_eq!(unique.len(), 10, "同一片段不能在两页里重复出现");

    // 旧游标不能重翻：拿第一页的游标再来一次是 cursor_expired。
    let stale = first.cursor.clone().unwrap();
    let error = core
        .search_next_page(&first.session_id, Some(&stale))
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::CursorExpired);
}

#[test]
fn cursor_from_another_session_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let mut core = new_core(dir.path());
    add_material(
        &mut core,
        "日记.txt",
        &ten_paragraphs(),
        Utc.with_ymd_and_hms(2026, 9, 20, 4, 0, 0).unwrap(),
        "op1",
    );

    let first = core.start_search(request("妈妈", 3), 1).unwrap();
    let other = core.start_search(request("妈妈", 3), 2).unwrap();
    let foreign = other.cursor.clone().unwrap();

    let error = core
        .search_next_page(&first.session_id, Some(&foreign))
        .unwrap_err();
    assert_eq!(
        error.code(),
        ErrorCode::CursorExpired,
        "只能在自己的会话里翻页"
    );
}

#[test]
fn index_change_invalidates_the_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut core = new_core(dir.path());
    add_material(
        &mut core,
        "日记.txt",
        &ten_paragraphs(),
        Utc.with_ymd_and_hms(2026, 9, 20, 4, 0, 0).unwrap(),
        "op1",
    );

    let session = core.start_search(request("妈妈", 3), 1).unwrap();

    // 会话期间又导入了新材料：索引变了，旧快照必须作废。
    add_material(
        &mut core,
        "新日记.txt",
        "妈妈又打了一次电话。\n\n第二段也提到妈妈。\n",
        Utc.with_ymd_and_hms(2026, 9, 21, 4, 0, 0).unwrap(),
        "op2",
    );

    let next = session.cursor.clone().unwrap();
    let error = core
        .search_next_page(&session.session_id, Some(&next))
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::SearchExpired);

    let error = core.search_snapshot(&session.session_id).unwrap_err();
    assert_eq!(error.code(), ErrorCode::SearchExpired);
}

#[test]
fn cancel_stops_paging_but_keeps_the_snapshot_readable() {
    let dir = tempfile::tempdir().unwrap();
    let mut core = new_core(dir.path());
    add_material(
        &mut core,
        "日记.txt",
        &ten_paragraphs(),
        Utc.with_ymd_and_hms(2026, 9, 20, 4, 0, 0).unwrap(),
        "op1",
    );

    let session = core.start_search(request("妈妈", 3), 1).unwrap();
    let cancelled = core.cancel_search(&session.session_id).unwrap();
    assert_eq!(cancelled.phase, SearchPhase::Cancelled);
    assert!(cancelled.cursor.is_none(), "取消后不该再给下一页游标");
    assert_eq!(
        cancelled.results.len(),
        session.results.len(),
        "取消不删除已经返回的结果"
    );

    // 快照仍可读（状态是「已取消」，不是「过期」）。
    let again = core.search_snapshot(&session.session_id).unwrap();
    assert_eq!(again.phase, SearchPhase::Cancelled);

    // 但不能再翻页。
    let cursor = session.cursor.clone().unwrap();
    let error = core
        .search_next_page(&session.session_id, Some(&cursor))
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::InvalidState);
}

#[test]
fn empty_query_and_unimplemented_modes_say_so() {
    let dir = tempfile::tempdir().unwrap();
    let mut core = new_core(dir.path());
    add_material(
        &mut core,
        "日记.txt",
        &ten_paragraphs(),
        Utc.with_ymd_and_hms(2026, 9, 20, 4, 0, 0).unwrap(),
        "op1",
    );

    let empty = core.start_search(request("   ", 10), 1).unwrap();
    assert!(empty.results.is_empty());
    assert_eq!(empty.phase, SearchPhase::Done);
    assert!(
        empty.warnings.iter().any(|warning| warning.contains("空")),
        "空查询要说清楚：{:?}",
        empty.warnings
    );

    let mut hybrid = request("妈妈", 10);
    hybrid.mode = SearchMode::Hybrid;
    hybrid.filters.include_old_diary_versions = true;
    let downgraded = core.start_search(hybrid, 2).unwrap();
    assert!(
        downgraded
            .warnings
            .iter()
            .any(|warning| warning.contains("关键词")),
        "语义/混合还没接，必须如实说降级到关键词：{:?}",
        downgraded.warnings
    );
    assert!(
        downgraded
            .warnings
            .iter()
            .any(|warning| warning.contains("历史版本")),
        "历史版本开关没有效果也要说：{:?}",
        downgraded.warnings
    );
    // 降级不等于不返回结果。
    assert!(!downgraded.results.is_empty());
    assert!(downgraded
        .results
        .iter()
        .all(|hit| hit.matched_by == vec![MatchedBy::Keyword]));
}

#[test]
fn filters_apply_to_day_range_scope_and_kind() {
    let dir = tempfile::tempdir().unwrap();
    let mut core = new_core(dir.path());
    let old_source = add_material(
        &mut core,
        "旧的.txt",
        "妈妈上个月来过。\n\n那天的饭菜很好。\n",
        Utc.with_ymd_and_hms(2026, 8, 10, 4, 0, 0).unwrap(),
        "op-old",
    );
    let new_source = add_material(
        &mut core,
        "新的.txt",
        "妈妈今天打过电话。\n\n她说下周再来。\n",
        Utc.with_ymd_and_hms(2026, 9, 20, 4, 0, 0).unwrap(),
        "op-new",
    );

    let all = core.start_search(request("妈妈", 10), 1).unwrap();
    assert_eq!(all.results.len(), 2);
    // 日期从近到远：新的排在前面。
    assert_eq!(
        all.results[0].source_id.as_deref(),
        Some(new_source.as_str())
    );

    let mut only_old = request("妈妈", 10);
    only_old.filters.to_day_key = Some("2026-09-01".to_owned());
    let filtered = core.start_search(only_old, 2).unwrap();
    assert_eq!(filtered.results.len(), 1);
    assert_eq!(
        filtered.results[0].source_id.as_deref(),
        Some(old_source.as_str())
    );

    let mut only_new = request("妈妈", 10);
    only_new.filters.from_day_key = Some("2026-09-01".to_owned());
    let filtered = core.start_search(only_new, 3).unwrap();
    assert_eq!(filtered.results.len(), 1);
    assert_eq!(
        filtered.results[0].source_id.as_deref(),
        Some(new_source.as_str())
    );

    let mut scoped = request("妈妈", 10);
    scoped.filters.source_scope = Some(vec![old_source.clone()]);
    let filtered = core.start_search(scoped, 4).unwrap();
    assert_eq!(filtered.results.len(), 1);
    assert_eq!(filtered.results[0].day_key.as_deref(), Some("2026-08-10"));

    // 今天能被索引的内容都是「文件」类型（文本导入走的是 file）；
    // 用 image 过滤应当一条都不剩，证明过滤真的到了 SQL 里。
    let mut images = request("妈妈", 10);
    images.filters.kinds = vec![SourceKind::Image];
    let filtered = core.start_search(images, 5).unwrap();
    assert!(filtered.results.is_empty());

    let mut files = request("妈妈", 10);
    files.filters.kinds = vec![SourceKind::File];
    let filtered = core.start_search(files, 6).unwrap();
    assert_eq!(filtered.results.len(), 2);
}

#[test]
fn snippet_and_highlights_point_at_the_match() {
    let dir = tempfile::tempdir().unwrap();
    let mut core = new_core(dir.path());
    add_material(
        &mut core,
        "长文.txt",
        &format!(
            "{}妈妈{}\n",
            "前面有很多铺垫".repeat(6),
            "后面还有很多内容".repeat(6)
        ),
        Utc.with_ymd_and_hms(2026, 9, 20, 4, 0, 0).unwrap(),
        "op1",
    );

    let snapshot = core.start_search(request("妈妈", 10), 1).unwrap();
    let hit = &snapshot.results[0];
    let snippet = hit.snippet.as_deref().unwrap();

    assert_eq!(hit.highlights.len(), 1);
    let range = hit.highlights[0];
    let highlighted: String = snippet
        .chars()
        .skip(range.start as usize)
        .take((range.end - range.start) as usize)
        .collect();
    assert_eq!(highlighted, "妈妈", "高亮区间必须正好落在查询词上");
    assert!(
        snippet.starts_with('…') && snippet.ends_with('…'),
        "两边都截断时要有省略号：{snippet}"
    );
    // 命中对象还要带上原件名与定位信息。
    assert_eq!(hit.title.as_deref(), Some("长文.txt"));
    assert!(hit.locator.is_some());
    assert_eq!(hit.source_kind, SourceKind::File);
}

#[test]
fn sessions_do_not_survive_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("library.sqlite");
    let session_id = {
        let mut core = Core::open(&path).unwrap();
        add_material(
            &mut core,
            "日记.txt",
            &ten_paragraphs(),
            Utc.with_ymd_and_hms(2026, 9, 20, 4, 0, 0).unwrap(),
            "op1",
        );
        core.start_search(request("妈妈", 3), 1).unwrap().session_id
    };

    let core = Core::open(&path).unwrap();
    let error = core.search_snapshot(&session_id).unwrap_err();
    assert_eq!(
        error.code(),
        ErrorCode::SearchExpired,
        "会话是内存态，重开后必须报过期而不是给旧结果"
    );
}

#[test]
fn unknown_session_reports_expired_not_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let core = new_core(dir.path());
    let error = core.search_snapshot("search_不存在").unwrap_err();
    assert_eq!(error.code(), ErrorCode::SearchExpired);
    assert!(error.retryable(), "契约给这两个码的处理是「重新发起」");
}