import 'dart:async';

import 'package:diary_api/diary_api.dart';
import 'package:flutter/foundation.dart';

enum SavePhase { ready, editing, saving, saved, failed }

/// F0 页面状态。所有记录操作只经过注入的 DiaryApi。
class CaptureController extends ChangeNotifier {
  CaptureController(
    this.api, {
    this.autoSaveDelay = const Duration(milliseconds: 450),
  });

  final DiaryApi api;
  final Duration autoSaveDelay;
  Timer? _debounce;
  Capture? _draft;
  Future<bool>? _saving;
  int _operation = 0;
  bool loading = true;
  String text = '';
  String _savedText = '';
  String? error;
  SavePhase phase = SavePhase.ready;
  List<Capture> recent = const [];

  String _newOperation() =>
      'f0-${++_operation}-${DateTime.now().microsecondsSinceEpoch}';

  Future<void> initialize({bool opened = false}) async {
    try {
      if (!opened) await api.open();
      await refresh();
    } catch (failure) {
      error = _message(failure);
      phase = SavePhase.failed;
    } finally {
      loading = false;
      notifyListeners();
    }
  }

  Future<void> refresh() async {
    final page = await api.listCaptures(pageSize: 20);
    recent = page.captures;
    notifyListeners();
  }

  void updateText(String value) {
    text = value;
    error = null;
    phase = value == _savedText ? SavePhase.saved : SavePhase.editing;
    _debounce?.cancel();
    if (value.trim().isNotEmpty && value != _savedText) {
      _debounce = Timer(autoSaveDelay, saveNow);
    }
    notifyListeners();
  }

  Future<bool> saveNow() async {
    _debounce?.cancel();
    if (text.trim().isEmpty) return false;
    if (_saving != null) {
      await _saving;
      if (text == _savedText) return true;
    }
    final requestedText = text;
    final task = _save(requestedText);
    _saving = task;
    try {
      return await task;
    } finally {
      _saving = null;
      if (text != requestedText && text.trim().isNotEmpty) {
        _debounce = Timer(autoSaveDelay, saveNow);
      }
    }
  }

  Future<bool> _save(String requestedText) async {
    phase = SavePhase.saving;
    error = null;
    notifyListeners();
    try {
      _draft ??= await api.createDraft(operationId: _newOperation());
      final result = await api.saveDraft(
        id: _draft!.id,
        text: requestedText,
        expectedRevision: _draft!.revision,
        operationId: _newOperation(),
      );
      if (!result.durable) {
        throw const DiaryException(
          code: DiaryErrorCode.unknown,
          message: '尚未收到保存确认。',
          retryable: true,
        );
      }
      _draft = _draft!.copyWith(
        revision: result.revision,
        draftText: requestedText,
        updatedAt: result.savedAt,
      );
      _savedText = requestedText;
      phase = text == _savedText ? SavePhase.saved : SavePhase.editing;
      await refresh();
      return true;
    } catch (failure) {
      phase = SavePhase.failed;
      error = _message(failure);
      notifyListeners();
      return false;
    } finally {
      notifyListeners();
    }
  }

  Future<bool> finish() async {
    _debounce?.cancel();
    if (text.trim().isEmpty) return false;
    if (!await saveNow()) return false;
    try {
      final done = await api.commit(
        id: _draft!.id,
        expectedRevision: _draft!.revision,
        operationId: _newOperation(),
      );
      _draft = null;
      text = '';
      _savedText = '';
      phase = SavePhase.ready;
      error = null;
      recent = [
        done.capture,
        ...recent.where((capture) => capture.id != done.capture.id),
      ];
      notifyListeners();
      return true;
    } catch (failure) {
      phase = SavePhase.failed;
      error = _message(failure);
      notifyListeners();
      return false;
    }
  }

  String _message(Object failure) =>
      failure is DiaryException ? failure.message : '这次操作没有完成，请稍后重试。';

  @override
  void dispose() {
    _debounce?.cancel();
    super.dispose();
  }
}
