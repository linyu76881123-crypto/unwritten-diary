//! 持久任务队列，契约第 4.4 节与任务书 7.1 节。
//!
//! 这一片只做**队列本身**：入队、领取、成功/失败、退避重试、取消、恢复、下次唤醒
//! 时刻与通知去重。任务具体做什么（转写、整理、索引）属于后面的切片；这里保证
//! 的是「队列重启后还在」和「失败不影响原件」。
//!
//! 优先级照任务书 7.1 节：保存与录音片段登记最高，整库重建最低。

use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, OptionalExtension, Transaction};

use crate::error::{CoreError, Result};
use crate::model::{Job, JobProgress, JobState, NewJob};
use crate::support;
use crate::Core;

/// 重试退避：30 秒起，每次翻倍，上限 1 小时。
///
/// 不用「无限重试」：次数上限由任务的 max_attempts 决定，超了就进 failed，
/// 由用户决定要不要手动重试。
fn backoff(attempt: i64) -> Duration {
    let factor = 1_i64 << attempt.clamp(0, 10) as u32;
    Duration::seconds(30_i64.saturating_mul(factor).min(3600))
}

pub(crate) fn enqueue_in_tx(tx: &Transaction<'_>, job: NewJob) -> Result<Job> {
    let now = support::now();
    let id = support::new_id("job");
    let target_ids = serde_json::to_string(&job.target_ids)?;
    let max_attempts = job.max_attempts.max(1);
    tx.execute(
        "INSERT INTO jobs (id, kind, state, priority, target_ids, input_snapshot_hash, \
         attempt_count, max_attempts, created_at, updated_at) \
         VALUES (?1, ?2, 'queued', ?3, ?4, ?5, 0, ?6, ?7, ?7)",
        params![
            id,
            job.kind,
            job.priority.value(),
            target_ids,
            job.input_snapshot_hash,
            max_attempts,
            support::to_iso(now),
        ],
    )?;
    Ok(Job {
        id,
        kind: job.kind,
        state: JobState::Queued,
        priority: job.priority.value(),
        target_ids: job.target_ids,
        input_snapshot_hash: job.input_snapshot_hash,
        progress: None,
        attempt_count: 0,
        max_attempts,
        next_attempt_at: None,
        error_code: None,
        requires_user_action: false,
        attention_key: None,
        created_at: now,
        updated_at: now,
    })
}

/// 入队一个任务。
pub(crate) fn enqueue(core: &mut Core, job: NewJob) -> Result<Job> {
    let tx = core.conn.transaction()?;
    let queued = enqueue_in_tx(&tx, job)?;
    tx.commit()?;
    Ok(queued)
}

pub(crate) fn get(core: &Core, job_id: &str) -> Result<Job> {
    load_job(&core.conn, job_id)
}

/// 按状态筛选列出任务。按优先级与创建时间排序。
pub(crate) fn list(core: &Core, states: Option<&[JobState]>, limit: usize) -> Result<Vec<Job>> {
    let page = limit.clamp(1, 200);
    let mut stmt = core.conn.prepare(
        "SELECT id FROM jobs WHERE (?1 IS NULL OR state IN (SELECT value FROM json_each(?1))) \
         ORDER BY priority DESC, created_at, id LIMIT ?2",
    )?;
    let filter = match states {
        Some(values) if !values.is_empty() => Some(serde_json::to_string(
            &values.iter().map(|state| state.wire()).collect::<Vec<_>>(),
        )?),
        _ => None,
    };
    let ids = stmt
        .query_map(params![filter, page as i64], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    ids.iter().map(|id| load_job(&core.conn, id)).collect()
}

/// 按状态精确计数。
///
/// 存在的理由：`list()` 会把 limit 夹到 200（分页用），拿它 `.len()` 当计数
/// 就会在任务多的时候少报。需要数字的地方一律走这里。
pub(crate) fn count(core: &Core, states: Option<&[JobState]>) -> Result<i64> {
    let filter = match states {
        Some(values) if !values.is_empty() => Some(serde_json::to_string(
            &values.iter().map(|state| state.wire()).collect::<Vec<_>>(),
        )?),
        _ => None,
    };
    let total = core.conn.query_row(
        "SELECT COUNT(*) FROM jobs WHERE (?1 IS NULL OR state IN (SELECT value FROM json_each(?1)))",
        params![filter],
        |row| row.get(0),
    )?;
    Ok(total)
}

/// 领取下一个到期的任务：优先级高的先跑，同优先级先来先跑。
pub(crate) fn claim_next_due(core: &mut Core, now: DateTime<Utc>) -> Result<Option<Job>> {
    let candidate: Option<String> = core
        .conn
        .query_row(
            "SELECT id FROM jobs WHERE state IN ('queued', 'retry_wait') \
             AND (next_attempt_at IS NULL OR next_attempt_at <= ?1) \
             ORDER BY priority DESC, created_at, id LIMIT 1",
            params![support::to_iso(now)],
            |row| row.get(0),
        )
        .optional()?;

    let Some(job_id) = candidate else {
        return Ok(None);
    };

    let job = load_job(&core.conn, &job_id)?;
    let attempt = job.attempt_count + 1;
    let now_text = support::to_iso(now);
    let tx = core.conn.transaction()?;
    tx.execute(
        "UPDATE jobs SET state = 'running', attempt_count = ?1, next_attempt_at = NULL, \
         updated_at = ?2 WHERE id = ?3",
        params![attempt, now_text, job_id],
    )?;
    tx.execute(
        "INSERT INTO job_attempts (job_id, attempt, started_at) VALUES (?1, ?2, ?3)",
        params![job_id, attempt, now_text],
    )?;
    tx.commit()?;

    Ok(Some(load_job(&core.conn, &job_id)?))
}

/// 任务成功。
pub(crate) fn complete(core: &mut Core, job_id: &str) -> Result<Job> {
    let job = load_job(&core.conn, job_id)?;
    if job.state.is_terminal() && job.state != JobState::Succeeded {
        return Err(CoreError::InvalidState {
            entity: "任务",
            id: job_id.to_owned(),
            state: job.state.wire().to_owned(),
        });
    }
    let now = support::to_iso(support::now());
    let tx = core.conn.transaction()?;
    tx.execute(
        "UPDATE jobs SET state = 'succeeded', error_code = NULL, requires_user_action = 0, \
         updated_at = ?1 WHERE id = ?2",
        params![now, job_id],
    )?;
    tx.execute(
        "UPDATE job_attempts SET finished_at = ?1, outcome = 'succeeded' \
         WHERE job_id = ?2 AND attempt = ?3",
        params![now, job_id, job.attempt_count],
    )?;
    tx.commit()?;
    load_job(&core.conn, job_id)
}

/// 任务失败：还有额度就退避重试，用完了就进 failed。
pub(crate) fn fail(
    core: &mut Core,
    job_id: &str,
    error_code: &str,
    message: &str,
    now: DateTime<Utc>,
) -> Result<Job> {
    let job = load_job(&core.conn, job_id)?;
    let exhausted = job.attempt_count >= job.max_attempts;
    let (state, next_attempt) = if exhausted {
        (JobState::Failed, None)
    } else {
        (
            JobState::RetryWait,
            Some(support::to_iso(now + backoff(job.attempt_count))),
        )
    };

    let now_text = support::to_iso(now);
    let tx = core.conn.transaction()?;
    tx.execute(
        "UPDATE jobs SET state = ?1, error_code = ?2, next_attempt_at = ?3, updated_at = ?4 \
         WHERE id = ?5",
        params![state.wire(), error_code, next_attempt, now_text, job_id],
    )?;
    tx.execute(
        "UPDATE job_attempts SET finished_at = ?1, outcome = ?2, error_code = ?3, message = ?4 \
         WHERE job_id = ?5 AND attempt = ?6",
        params![
            now_text,
            state.wire(),
            error_code,
            message,
            job_id,
            job.attempt_count
        ],
    )?;
    tx.commit()?;
    load_job(&core.conn, job_id)
}

/// 手动重试。用完了额度就多给一次机会，并如实记录下来。
pub(crate) fn retry(core: &mut Core, job_id: &str, operation_id: &str) -> Result<Job> {
    let fingerprint = support::fingerprint(&["jobs.retry", job_id]);
    if let Some(json) = core.receipt("jobs.retry", &fingerprint, operation_id)? {
        return Ok(serde_json::from_str(&json)?);
    }

    let job = load_job(&core.conn, job_id)?;
    if job.state == JobState::Running {
        return Err(CoreError::InvalidState {
            entity: "任务",
            id: job_id.to_owned(),
            state: "正在运行，不能重试".to_owned(),
        });
    }
    let max_attempts = if job.attempt_count >= job.max_attempts {
        job.attempt_count + 1
    } else {
        job.max_attempts
    };

    let tx = core.conn.transaction()?;
    tx.execute(
        "UPDATE jobs SET state = 'queued', next_attempt_at = NULL, max_attempts = ?1, \
         error_code = NULL, updated_at = ?2 WHERE id = ?3",
        params![max_attempts, support::to_iso(support::now()), job_id],
    )?;
    let updated = load_job(&tx, job_id)?;
    crate::store_receipt(
        &tx,
        operation_id,
        "jobs.retry",
        &fingerprint,
        &serde_json::to_string(&updated)?,
    )?;
    tx.commit()?;
    Ok(updated)
}

/// 取消任务。取消不删除任何原件。
pub(crate) fn cancel(core: &mut Core, job_id: &str, operation_id: &str) -> Result<Job> {
    let fingerprint = support::fingerprint(&["jobs.cancel", job_id]);
    if let Some(json) = core.receipt("jobs.cancel", &fingerprint, operation_id)? {
        return Ok(serde_json::from_str(&json)?);
    }

    let job = load_job(&core.conn, job_id)?;
    if job.state.is_terminal() {
        return Err(CoreError::InvalidState {
            entity: "任务",
            id: job_id.to_owned(),
            state: job.state.wire().to_owned(),
        });
    }

    let now = support::to_iso(support::now());
    let tx = core.conn.transaction()?;
    tx.execute(
        "UPDATE jobs SET state = 'cancelled', next_attempt_at = NULL, updated_at = ?1 WHERE id = ?2",
        params![now, job_id],
    )?;
    tx.execute(
        "UPDATE job_attempts SET finished_at = ?1, outcome = 'cancelled' \
         WHERE job_id = ?2 AND finished_at IS NULL",
        params![now, job_id],
    )?;
    let updated = load_job(&tx, job_id)?;
    crate::store_receipt(
        &tx,
        operation_id,
        "jobs.cancel",
        &fingerprint,
        &serde_json::to_string(&updated)?,
    )?;
    tx.commit()?;
    Ok(updated)
}

/// 建议的下次唤醒时刻；没有待办时为 None。
pub(crate) fn next_wakeup(core: &Core) -> Result<Option<DateTime<Utc>>> {
    let now = support::now();
    // 有立刻能跑的排队任务，就该现在唤醒。
    let ready: i64 = core.conn.query_row(
        "SELECT COUNT(*) FROM jobs WHERE state = 'queued' AND next_attempt_at IS NULL",
        [],
        |row| row.get(0),
    )?;
    if ready > 0 {
        return Ok(Some(now));
    }

    let earliest: Option<String> = core
        .conn
        .query_row(
            "SELECT MIN(next_attempt_at) FROM jobs WHERE state IN ('queued', 'retry_wait') \
             AND next_attempt_at IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    match earliest {
        Some(text) => Ok(Some(support::parse_iso(&text)?)),
        None => Ok(None),
    }
}

/// 记录一个需要用户处理的问题已经展示过，避免每次重开重复通知。
pub(crate) fn acknowledge_attention(core: &mut Core, attention_key: &str) -> Result<()> {
    core.conn.execute(
        "INSERT INTO attention_acks (attention_key, acknowledged_at) VALUES (?1, ?2) \
         ON CONFLICT(attention_key) DO UPDATE SET acknowledged_at = excluded.acknowledged_at",
        params![attention_key, support::to_iso(support::now())],
    )?;
    Ok(())
}

pub(crate) fn is_attention_acknowledged(core: &Core, attention_key: &str) -> Result<bool> {
    let found: Option<i64> = core
        .conn
        .query_row(
            "SELECT 1 FROM attention_acks WHERE attention_key = ?1",
            params![attention_key],
            |row| row.get(0),
        )
        .optional()?;
    Ok(found.is_some())
}

// ------------------------------------------------------------------ 内部

fn load_job(conn: &rusqlite::Connection, job_id: &str) -> Result<Job> {
    let row = conn
        .query_row(
            "SELECT id, kind, state, priority, target_ids, input_snapshot_hash, \
             progress_completed, progress_total, attempt_count, max_attempts, next_attempt_at, \
             error_code, requires_user_action, attention_key, created_at, updated_at \
             FROM jobs WHERE id = ?1",
            params![job_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                    row.get::<_, Option<i64>>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, i64>(12)?,
                    row.get::<_, Option<String>>(13)?,
                    row.get::<_, String>(14)?,
                    row.get::<_, String>(15)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| CoreError::NotFound {
            entity: "任务",
            id: job_id.to_owned(),
        })?;

    let progress = match (row.6, row.7) {
        // 没有可靠进度时必须是 None：不虚构百分比。
        (Some(completed), Some(total)) if total > 0 => Some(JobProgress { completed, total }),
        _ => None,
    };

    Ok(Job {
        id: row.0,
        kind: row.1,
        state: JobState::from_wire(&row.2).ok_or_else(|| CoreError::CorruptedData {
            message: format!("未知任务状态：{}", row.2),
        })?,
        priority: row.3,
        target_ids: serde_json::from_str(&row.4)?,
        input_snapshot_hash: row.5,
        progress,
        attempt_count: row.8,
        max_attempts: row.9,
        next_attempt_at: row.10.map(|text| support::parse_iso(&text)).transpose()?,
        error_code: row.11,
        requires_user_action: row.12 != 0,
        attention_key: row.13,
        created_at: support::parse_iso(&row.14)?,
        updated_at: support::parse_iso(&row.15)?,
    })
}