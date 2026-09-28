//! 契约第 2 节的数据对象（B1a 用到的部分）与调用结果。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 记录状态，契约第 3 节的草稿状态机。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureState {
    Draft,
    Committed,
    Trashed,
}

impl CaptureState {
    pub fn wire(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Committed => "committed",
            Self::Trashed => "trashed",
        }
    }

    pub fn from_wire(value: &str) -> Option<Self> {
        match value {
            "draft" => Some(Self::Draft),
            "committed" => Some(Self::Committed),
            "trashed" => Some(Self::Trashed),
            _ => None,
        }
    }
}

/// 原始内容的作者，契约第 2.2 节。AI 生成的正文不写成 user。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorType {
    User,
    Import,
}

impl AuthorType {
    pub fn wire(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Import => "import",
        }
    }

    pub fn from_wire(value: &str) -> Option<Self> {
        match value {
            "user" => Some(Self::User),
            "import" => Some(Self::Import),
            _ => None,
        }
    }
}

/// 未处理 / 处理中 / 可检索 / 需关注的数量。
///
/// B1a 还没有提取与索引，所以这些数字现在恒为 0；等 B1b/B2 接上后再算。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ProcessingSummary {
    pub unprocessed: u32,
    pub processing: u32,
    pub searchable: u32,
    pub needs_attention: u32,
}

/// 一次混合记录，契约第 2.1 节。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capture {
    pub id: String,
    pub revision: i64,
    pub state: CaptureState,
    pub occurred_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub time_zone: String,
    pub utc_offset_minutes: i32,
    pub day_key: String,
    pub ordered_source_ids: Vec<String>,
    pub draft_text: String,
    pub processing_summary: ProcessingSummary,
}

/// 一条记录里的原始内容条目，契约第 2.2 节。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceItem {
    pub source_id: String,
    pub capture_id: String,
    pub kind: String,
    pub position: i64,
    pub current_revision_id: Option<String>,
}

/// 原始文字或来源元数据的某一次修订。旧版本永远保留。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRevision {
    pub revision_id: String,
    pub source_id: String,
    pub parent_revision_id: Option<String>,
    pub text: Option<String>,
    pub asset_id: Option<String>,
    pub author_type: AuthorType,
    pub occurred_at: DateTime<Utc>,
}

/// 草稿保存结果。只有 `durable` 为真，界面才能显示「已保存」。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftSaveResult {
    pub revision: i64,
    pub durable: bool,
    pub saved_at: DateTime<Utc>,
}

/// 提交结果：记录本身与提交时创建的原始文字版本。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitResult {
    pub capture: Capture,
    pub original_text_revision: Option<SourceRevision>,
}

/// 记录分页结果。cursor 对调用方不透明。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturePage {
    pub captures: Vec<Capture>,
    pub next_cursor: Option<String>,
}

/// 持久业务事件类型，契约第 6 节。B1a 只发 capture.changed。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    CaptureChanged,
    AssetChanged,
}

impl EventType {
    pub fn wire(self) -> &'static str {
        match self {
            Self::CaptureChanged => "capture.changed",
            Self::AssetChanged => "asset.changed",
        }
    }

    pub fn from_wire(value: &str) -> Option<Self> {
        match value {
            "capture.changed" => Some(Self::CaptureChanged),
            "asset.changed" => Some(Self::AssetChanged),
            _ => None,
        }
    }
}

/// 持久业务事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainEvent {
    pub event_id: String,
    /// 单调递增的游标序号。
    pub sequence: i64,
    #[serde(rename = "type")]
    pub event_type: EventType,
    pub entity_id: String,
    pub revision: i64,
    pub emitted_at: DateTime<Utc>,
}