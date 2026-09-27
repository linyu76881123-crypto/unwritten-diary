/// 契约第 6 节的持久业务事件。
library;

import 'values.dart';

/// 持久业务事件。前端从快照开始订阅，按 revision 去重。
class DomainEvent {
  const DomainEvent({
    required this.eventId,
    required this.sequence,
    required this.type,
    required this.entityId,
    required this.revision,
    required this.emittedAt,
  });

  final String eventId;

  /// 单调递增的游标序号；游标过期时返回 cursor_expired。
  final int sequence;

  final DomainEventType type;
  final String entityId;
  final int revision;
  final DateTime emittedAt;

  @override
  bool operator ==(Object other) =>
      other is DomainEvent &&
      other.eventId == eventId &&
      other.sequence == sequence &&
      other.type == type &&
      other.entityId == entityId &&
      other.revision == revision &&
      other.emittedAt == emittedAt;

  @override
  int get hashCode =>
      Object.hash(eventId, sequence, type, entityId, revision, emittedAt);

  @override
  String toString() =>
      'DomainEvent($sequence, ${type.wire}, $entityId@$revision)';
}

/// 事件游标失效时抛出 `cursor_expired`，前端应重新读快照而不是无限等待。
class CursorExpiredException extends DiaryException {
  const CursorExpiredException({required super.message, super.jobId})
      : super(code: DiaryErrorCode.cursorExpired, retryable: true);
}

/// 搜索会话过期时抛出 `search_expired`，前端应重新发起查询。
class SearchExpiredException extends DiaryException {
  const SearchExpiredException({required super.message, super.jobId})
      : super(code: DiaryErrorCode.searchExpired, retryable: true);
}