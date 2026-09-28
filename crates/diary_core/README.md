# diary_core

设备内本地核心。当前完成到 **B1b**：资料库、迁移、幂等写入、记录路径，以及原件文件库与导入交接。

设计决定与未完成项记在 [`docs/architecture/m1-存储与幂等.md`](../../docs/architecture/m1-存储与幂等.md) 与 [`docs/architecture/m1-原件文件库.md`](../../docs/architecture/m1-原件文件库.md)。

## 已实现

| 能力 | 说明 |
|---|---|
| SQLite schema v1 → v2 + 迁移 | 版本记在 `PRAGMA user_version`，迁移在事务里，失败整体回退；旧构建遇到更高版本直接拒绝打开 |
| 记录路径 | `create_draft` / `save_draft` / `commit` / `get_capture` / `list_captures` |
| 原始文字修订 | `revise_text`：新建版本并保留旧版本与父版本引用 |
| 幂等 | `operation_receipts` + 请求指纹；重复提交返回原结果，换内容报冲突 |
| 乐观锁 | `expectedRevision` 不匹配报 `revision_conflict`，不静默覆盖 |
| 域事件 | `domain_events` 与业务写入同事务 |
| 原件文件库 | 内容按 sha256 存放与去重；`imports.prepare` / `finish` / `status` / `cancel` |
| 资产租约 | `assets.open` / `release`，内存态 + 过期回收；文件丢失如实报 `asset_missing` |
| 导入恢复 | 重开资料库时把未完成的导入标成 `recoverable`，不假装成功 |
| 错误码 | 映射到契约第 7 节，含 `SQLITE_FULL` → `storage_full`、ENOSPC → `storage_full` |

## 还没实现

录音会话与队列（B1c）、提取/索引/搜索（B2）、插件、备份恢复、桥接接线。

## 校验

```bash
cargo test -p diary_core        # 30 项：12 文件库 + 16 记录 + 2 单测
cargo clippy -p diary_core --all-targets -- -D warnings
```

MSRV 是 workspace 里钉的 1.94（见根 `Cargo.toml`）。大文件测试只在 Linux 上跑（要读 `/proc/self/status` 的峰值内存）。