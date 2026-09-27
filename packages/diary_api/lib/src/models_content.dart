/// 契约第 2.1–2.4 节的数据对象：记录、原始内容、资产与派生内容。
///
/// 全部为不可变值对象。ID 都是核心生成的不透明字符串，前端不得解析或拼接。
library;

import 'equality.dart';
import 'values.dart';

/// 未处理 / 处理中 / 可检索 / 需关注的数量，用于界面显示真实状态。
class ProcessingSummary {
  const ProcessingSummary({
    this.unprocessed = 0,
    this.processing = 0,
    this.searchable = 0,
    this.needsAttention = 0,
  });

  final int unprocessed;
  final int processing;
  final int searchable;
  final int needsAttention;

  ProcessingSummary copyWith({
    int? unprocessed,
    int? processing,
    int? searchable,
    int? needsAttention,
  }) =>
      ProcessingSummary(
        unprocessed: unprocessed ?? this.unprocessed,
        processing: processing ?? this.processing,
        searchable: searchable ?? this.searchable,
        needsAttention: needsAttention ?? this.needsAttention,
      );

  @override
  bool operator ==(Object other) =>
      other is ProcessingSummary &&
      other.unprocessed == unprocessed &&
      other.processing == processing &&
      other.searchable == searchable &&
      other.needsAttention == needsAttention;

  @override
  int get hashCode =>
      Object.hash(unprocessed, processing, searchable, needsAttention);

  @override
  String toString() =>
      'ProcessingSummary(unprocessed: $unprocessed, processing: $processing, '
      'searchable: $searchable, needsAttention: $needsAttention)';
}

/// 一次混合记录，可以同时包含文字、照片、录音和文件。契约第 2.1 节。
class Capture {
  const Capture({
    required this.id,
    required this.revision,
    required this.state,
    required this.occurredAt,
    required this.createdAt,
    required this.updatedAt,
    required this.timeZone,
    required this.utcOffsetMinutes,
    required this.dayKey,
    this.orderedSourceIds = const [],
    this.draftText = '',
    this.processingSummary = const ProcessingSummary(),
  });

  final String id;
  final int revision;
  final CaptureState state;

  /// 事件时间，UTC 语义。
  final DateTime occurredAt;

  /// 入库时间与最后修改时间。
  final DateTime createdAt;
  final DateTime updatedAt;

  /// IANA 时区名与当时的偏移分钟数。
  final String timeZone;
  final int utcOffsetMinutes;

  /// 日记归属日，YYYY-MM-DD，按设置的 diaryTimeZone 计算。
  final String dayKey;

  /// 按展示顺序组织的文字或附件来源。
  final List<String> orderedSourceIds;

  /// 草稿正文，仅 draft 阶段可直接变更。
  final String draftText;

  final ProcessingSummary processingSummary;

  Capture copyWith({
    int? revision,
    CaptureState? state,
    DateTime? occurredAt,
    DateTime? updatedAt,
    String? timeZone,
    int? utcOffsetMinutes,
    String? dayKey,
    List<String>? orderedSourceIds,
    String? draftText,
    ProcessingSummary? processingSummary,
  }) =>
      Capture(
        id: id,
        revision: revision ?? this.revision,
        state: state ?? this.state,
        occurredAt: occurredAt ?? this.occurredAt,
        createdAt: createdAt,
        updatedAt: updatedAt ?? this.updatedAt,
        timeZone: timeZone ?? this.timeZone,
        utcOffsetMinutes: utcOffsetMinutes ?? this.utcOffsetMinutes,
        dayKey: dayKey ?? this.dayKey,
        orderedSourceIds: orderedSourceIds ?? this.orderedSourceIds,
        draftText: draftText ?? this.draftText,
        processingSummary: processingSummary ?? this.processingSummary,
      );

  @override
  bool operator ==(Object other) =>
      other is Capture &&
      other.id == id &&
      other.revision == revision &&
      other.state == state &&
      other.occurredAt == occurredAt &&
      other.createdAt == createdAt &&
      other.updatedAt == updatedAt &&
      other.timeZone == timeZone &&
      other.utcOffsetMinutes == utcOffsetMinutes &&
      other.dayKey == dayKey &&
      listEquals(other.orderedSourceIds, orderedSourceIds) &&
      other.draftText == draftText &&
      other.processingSummary == processingSummary;

  @override
  int get hashCode => Object.hash(
        id,
        revision,
        state,
        occurredAt,
        createdAt,
        updatedAt,
        timeZone,
        utcOffsetMinutes,
        dayKey,
        Object.hashAll(orderedSourceIds),
        draftText,
        processingSummary,
      );

  @override
  String toString() => 'Capture($id, $state, revision: $revision, day: $dayKey)';
}

/// 导入附带的来源信息。不得冒充已确认事实。契约第 2.2 节。
class Provenance {
  const Provenance({
    this.origin,
    this.description,
    this.externalReference,
  });

  final ImportOrigin? origin;
  final String? description;

  /// 外部路径或 content URI 之类的线索。不作为长期资产地址。
  final String? externalReference;

  @override
  bool operator ==(Object other) =>
      other is Provenance &&
      other.origin == origin &&
      other.description == description &&
      other.externalReference == externalReference;

  @override
  int get hashCode => Object.hash(origin, description, externalReference);

  @override
  String toString() => 'Provenance($origin, $description)';
}

/// 一条记录里的原始内容条目。契约第 2.2 节。
class SourceItem {
  const SourceItem({
    required this.sourceId,
    required this.captureId,
    required this.kind,
    this.currentRevisionId,
  });

  final String sourceId;
  final String captureId;
  final SourceKind kind;
  final String? currentRevisionId;

  @override
  bool operator ==(Object other) =>
      other is SourceItem &&
      other.sourceId == sourceId &&
      other.captureId == captureId &&
      other.kind == kind &&
      other.currentRevisionId == currentRevisionId;

  @override
  int get hashCode => Object.hash(sourceId, captureId, kind, currentRevisionId);

  @override
  String toString() => 'SourceItem($sourceId, $kind)';
}

/// 原始文字或来源元数据的某一次修订。旧版本永远保留。契约第 2.2 节。
class SourceRevision {
  const SourceRevision({
    required this.revisionId,
    required this.sourceId,
    this.parentRevisionId,
    this.text,
    this.assetId,
    required this.authorType,
    required this.occurredAt,
    this.provenance,
  });

  final String revisionId;
  final String sourceId;
  final String? parentRevisionId;

  /// 文字原件；与 [assetId] 二者之一按 [SourceItem.kind] 决定。
  final String? text;
  final String? assetId;

  /// AI 生成的正文不写成 user 来源。
  final AuthorType authorType;
  final DateTime occurredAt;
  final Provenance? provenance;

  @override
  bool operator ==(Object other) =>
      other is SourceRevision &&
      other.revisionId == revisionId &&
      other.sourceId == sourceId &&
      other.parentRevisionId == parentRevisionId &&
      other.text == text &&
      other.assetId == assetId &&
      other.authorType == authorType &&
      other.occurredAt == occurredAt &&
      other.provenance == provenance;

  @override
  int get hashCode => Object.hash(
        revisionId,
        sourceId,
        parentRevisionId,
        text,
        assetId,
        authorType,
        occurredAt,
        provenance,
      );

  @override
  String toString() => 'SourceRevision($revisionId of $sourceId)';
}

/// 文件库中的资产。契约第 2.3 节。
class Asset {
  const Asset({
    required this.id,
    required this.objectRef,
    required this.originalName,
    required this.detectedMime,
    required this.byteSize,
    required this.sha256,
    required this.storageState,
    required this.importOrigin,
    required this.createdAt,
    this.mediaDurationMs,
    this.width,
    this.height,
  });

  final String id;

  /// 核心管理的内部文件引用。不是供插件任意访问的系统路径。
  final String objectRef;

  final String originalName;
  final String detectedMime;
  final int byteSize;
  final String sha256;
  final AssetStorageState storageState;
  final ImportOrigin importOrigin;
  final DateTime createdAt;

  /// 可选媒体属性。
  final int? mediaDurationMs;
  final int? width;
  final int? height;

  Asset copyWith({
    String? objectRef,
    String? originalName,
    String? detectedMime,
    int? byteSize,
    String? sha256,
    AssetStorageState? storageState,
    ImportOrigin? importOrigin,
    int? mediaDurationMs,
    int? width,
    int? height,
  }) =>
      Asset(
        id: id,
        objectRef: objectRef ?? this.objectRef,
        originalName: originalName ?? this.originalName,
        detectedMime: detectedMime ?? this.detectedMime,
        byteSize: byteSize ?? this.byteSize,
        sha256: sha256 ?? this.sha256,
        storageState: storageState ?? this.storageState,
        importOrigin: importOrigin ?? this.importOrigin,
        createdAt: createdAt,
        mediaDurationMs: mediaDurationMs ?? this.mediaDurationMs,
        width: width ?? this.width,
        height: height ?? this.height,
      );

  @override
  bool operator ==(Object other) =>
      other is Asset &&
      other.id == id &&
      other.objectRef == objectRef &&
      other.originalName == originalName &&
      other.detectedMime == detectedMime &&
      other.byteSize == byteSize &&
      other.sha256 == sha256 &&
      other.storageState == storageState &&
      other.importOrigin == importOrigin &&
      other.createdAt == createdAt &&
      other.mediaDurationMs == mediaDurationMs &&
      other.width == width &&
      other.height == height;

  @override
  int get hashCode => Object.hash(
        id,
        objectRef,
        originalName,
        detectedMime,
        byteSize,
        sha256,
        storageState,
        importOrigin,
        createdAt,
        mediaDurationMs,
        width,
        height,
      );

  @override
  String toString() => 'Asset($id, $originalName, $storageState)';
}

/// 图片定位用的归一化矩形，取值 0–1。契约第 2.4 节。
class NormalizedRect {
  const NormalizedRect({
    required this.left,
    required this.top,
    required this.right,
    required this.bottom,
  });

  final double left;
  final double top;
  final double right;
  final double bottom;

  @override
  bool operator ==(Object other) =>
      other is NormalizedRect &&
      other.left == left &&
      other.top == top &&
      other.right == right &&
      other.bottom == bottom;

  @override
  int get hashCode => Object.hash(left, top, right, bottom);

  @override
  String toString() => 'NormalizedRect($left, $top, $right, $bottom)';
}

/// 原文中的字符区间。Unicode 标量值计数，左闭右开。契约第 1.2 节。
class TextRange {
  const TextRange({required this.start, required this.end});

  final int start;
  final int end;

  @override
  bool operator ==(Object other) =>
      other is TextRange && other.start == start && other.end == end;

  @override
  int get hashCode => Object.hash(start, end);

  @override
  String toString() => 'TextRange($start, $end)';
}

/// 定位信息。每个 locator 必须携带 sourceRevisionId，保证定位到当时的原文版本。
/// 契约第 2.4 节。
class SourceLocator {
  const SourceLocator({
    required this.type,
    required this.sourceRevisionId,
    this.textRange,
    this.startMs,
    this.endMs,
    this.pageNumber,
    this.blockId,
    this.rect,
    this.assetId,
  });

  const SourceLocator.text({
    required String sourceRevisionId,
    required TextRange range,
  }) : this(
          type: LocatorType.textRange,
          sourceRevisionId: sourceRevisionId,
          textRange: range,
        );

  const SourceLocator.media({
    required String sourceRevisionId,
    required bool isVideo,
    required int fromMs,
    required int toMs,
  }) : this(
          type: isVideo ? LocatorType.video : LocatorType.audio,
          sourceRevisionId: sourceRevisionId,
          startMs: fromMs,
          endMs: toMs,
        );

  final LocatorType type;
  final String sourceRevisionId;

  final TextRange? textRange;

  /// 音视频位置：从逻辑媒体开始的毫秒数，暂停间隔不计入播放时间。
  final int? startMs;
  final int? endMs;

  /// 页码从 1 开始；无法可靠定位时返回空，不伪造。
  final int? pageNumber;
  final String? blockId;

  final NormalizedRect? rect;
  final String? assetId;

  @override
  bool operator ==(Object other) =>
      other is SourceLocator &&
      other.type == type &&
      other.sourceRevisionId == sourceRevisionId &&
      other.textRange == textRange &&
      other.startMs == startMs &&
      other.endMs == endMs &&
      other.pageNumber == pageNumber &&
      other.blockId == blockId &&
      other.rect == rect &&
      other.assetId == assetId;

  @override
  int get hashCode => Object.hash(
        type,
        sourceRevisionId,
        textRange,
        startMs,
        endMs,
        pageNumber,
        blockId,
        rect,
        assetId,
      );

  @override
  String toString() => 'SourceLocator(${type.wire}, rev: $sourceRevisionId)';
}

/// 派生内容中的一个片段。契约第 2.4 节。
class ExtractedSegment {
  const ExtractedSegment({
    required this.text,
    required this.locator,
    this.coverage = Coverage.complete,
  });

  final String text;
  final SourceLocator locator;
  final Coverage coverage;

  @override
  bool operator ==(Object other) =>
      other is ExtractedSegment &&
      other.text == text &&
      other.locator == locator &&
      other.coverage == coverage;

  @override
  int get hashCode => Object.hash(text, locator, coverage);

  @override
  String toString() => 'ExtractedSegment(${text.length} chars)';
}

/// 可重建的派生内容，与原件分开保存。契约第 2.4 节。
class ExtractedContent {
  const ExtractedContent({
    required this.id,
    required this.sourceId,
    required this.sourceRevisionId,
    required this.extractorId,
    required this.extractorVersion,
    required this.status,
    required this.coverage,
    this.text,
    this.segments = const [],
    this.errorCode,
    this.coverageReason,
  });

  final String id;
  final String sourceId;
  final String sourceRevisionId;
  final String extractorId;
  final String extractorVersion;
  final ProcessingStatus status;
  final Coverage coverage;

  final String? text;
  final List<ExtractedSegment> segments;
  final DiaryErrorCode? errorCode;

  /// 用户可读的覆盖原因，例如「没有 OCR 能力，仅文件信息可搜」。
  final String? coverageReason;

  @override
  bool operator ==(Object other) =>
      other is ExtractedContent &&
      other.id == id &&
      other.sourceId == sourceId &&
      other.sourceRevisionId == sourceRevisionId &&
      other.extractorId == extractorId &&
      other.extractorVersion == extractorVersion &&
      other.status == status &&
      other.coverage == coverage &&
      other.text == text &&
      listEquals(other.segments, segments) &&
      other.errorCode == errorCode &&
      other.coverageReason == coverageReason;

  @override
  int get hashCode => Object.hash(
        id,
        sourceId,
        sourceRevisionId,
        extractorId,
        extractorVersion,
        status,
        coverage,
        text,
        Object.hashAll(segments),
        errorCode,
        coverageReason,
      );

  @override
  String toString() =>
      'ExtractedContent($id, ${status.wire}, coverage: ${coverage.wire})';
}