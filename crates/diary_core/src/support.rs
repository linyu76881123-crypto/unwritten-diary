//! 内部小工具：ID 生成、时间格式、日归属、请求指纹。

use chrono::{DateTime, FixedOffset, SecondsFormat, Utc};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::{CoreError, Result};

/// 生成不透明 ID。前缀只用于人和日志辨认，调用方不得解析。
pub fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::now_v7().simple())
}

/// UTC 时间的统一存储格式：带 Z 的 RFC 3339，毫秒精度。
pub fn to_iso(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub fn parse_iso(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|parsed| parsed.with_timezone(&Utc))
        .map_err(|err| CoreError::CorruptedData {
            message: format!("时间字段无法解析：{value}（{err}）"),
        })
}

/// 日归属：按事件时间加上当时偏移得到的本地日期，YYYY-MM-DD。
///
/// 契约第 1.2 节要求 dayKey 按 diaryTimeZone 计算。B1a 接收调用方给出的
/// 偏移（平台知道设备当前的偏移），**尚未**支持用 IANA 时区名去解析历史时刻
/// 的偏移——那需要时区数据库，留给导入路径处理。
pub fn day_key(at: DateTime<Utc>, offset_minutes: i32) -> String {
    let offset = FixedOffset::east_opt(offset_minutes * 60)
        .unwrap_or_else(|| FixedOffset::east_opt(0).expect("零偏移总是合法"));
    at.with_timezone(&offset)
        .date_naive()
        .format("%Y-%m-%d")
        .to_string()
}

/// 请求指纹：把操作名与关键入参哈希成一个稳定字符串。
///
/// 幂等回执用它区分「同一个操作的重复提交」与「同一个 operationId 被换了内容」。
pub fn fingerprint(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0x1f]);
    }
    format!("{:x}", hasher.finalize())
}

/// 从 `occurred_at|id` 形式的游标里拆出两部分。
pub fn split_cursor(cursor: &str) -> Result<(DateTime<Utc>, String)> {
    let (at, id) = cursor.split_once('|').ok_or_else(|| CoreError::CorruptedData {
        message: format!("游标格式不正确：{cursor}"),
    })?;
    Ok((parse_iso(at)?, id.to_owned()))
}

pub fn join_cursor(at: DateTime<Utc>, id: &str) -> String {
    format!("{}|{id}", to_iso(at))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn day_key_uses_local_date_with_offset() {
        // 2026-09-27T16:30Z 在东八区是 09-28 00:30。
        let at = Utc.with_ymd_and_hms(2026, 9, 27, 16, 30, 0).unwrap();
        assert_eq!(day_key(at, 480), "2026-09-28");
        assert_eq!(day_key(at, 0), "2026-09-27");
        // 西五区是 09-27 11:30。
        assert_eq!(day_key(at, -300), "2026-09-27");
    }

    #[test]
    fn fingerprint_is_stable_and_sensitive() {
        let a = fingerprint(&["commit", "cap_1", "3"]);
        let b = fingerprint(&["commit", "cap_1", "3"]);
        let c = fingerprint(&["commit", "cap_1", "4"]);
        assert_eq!(a, b);
        assert_ne!(a, c);
        // 分隔符避免字段拼接歧义："ab"+"c" 与 "a"+"bc" 必须不同。
        assert_ne!(fingerprint(&["ab", "c"]), fingerprint(&["a", "bc"]));
    }
}