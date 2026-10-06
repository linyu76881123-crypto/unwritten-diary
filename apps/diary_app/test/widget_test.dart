import 'package:diary_api/diary_api.dart';
import 'package:diary_mock/diary_mock.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:unwritten_diary/main.dart';

Future<void> enterDiary(WidgetTester tester) async {
  expect(find.text('把今天，\n轻轻收好。'), findsOneWidget);
  await tester.ensureVisible(find.byKey(const Key('enter-diary')));
  await tester.tap(find.byKey(const Key('enter-diary')));
  await tester.pump();
  expect(find.text('正在收拢今天的光'), findsOneWidget);
  await tester.pump(const Duration(milliseconds: 1400));
  await tester.pumpAndSettle();
}

class CountingMockDiaryApi extends MockDiaryApi {
  int openCount = 0;

  @override
  Future<CoreSnapshot> open({String? libraryHandle}) {
    openCount++;
    return super.open(libraryHandle: libraryHandle);
  }
}

void main() {
  testWidgets('首次打开可输入，模拟保存后完成一条记录', (tester) async {
    final api = CountingMockDiaryApi();
    await tester.pumpWidget(DiaryApp(api: api));
    await enterDiary(tester);
    expect(api.openCount, 1);

    expect(find.text('今天，想留住什么？'), findsOneWidget);
    expect(find.textContaining('关闭应用后会消失'), findsOneWidget);
    await tester.enterText(find.byKey(const Key('capture-editor')), '今天的风很轻。');
    await tester.pump(const Duration(seconds: 1));
    await tester.pumpAndSettle();
    expect(find.textContaining('已模拟保存'), findsOneWidget);

    await tester.ensureVisible(find.byKey(const Key('finish-capture')));
    await tester.tap(find.byKey(const Key('finish-capture')));
    await tester.pumpAndSettle();
    expect(
      (await api.listCaptures()).captures.single.state,
      CaptureState.committed,
    );
    expect(find.text('今天的风很轻。'), findsOneWidget);
  });

  testWidgets('中文输入的换行不会提交；保存失败可重试', (tester) async {
    final api = MockDiaryApi()
      ..nextSaveFailure = const DiaryException(
        code: DiaryErrorCode.storageFull,
        message: '空间不足，尚未保存。',
      );
    await tester.pumpWidget(DiaryApp(api: api));
    await enterDiary(tester);
    final field = find.byKey(const Key('capture-editor'));
    await tester.enterText(field, '第一句');
    await tester.testTextInput.receiveAction(TextInputAction.newline);
    await tester.pump(const Duration(seconds: 1));
    await tester.pumpAndSettle();
    expect(
      (await api.listCaptures()).captures.single.state,
      CaptureState.draft,
    );
    expect(find.textContaining('空间不足'), findsOneWidget);

    await tester.ensureVisible(find.text('重试'));
    await tester.tap(find.text('重试'));
    await tester.pumpAndSettle();
    expect(find.textContaining('已模拟保存'), findsOneWidget);
    expect(
      (await api.listCaptures()).captures.single.state,
      CaptureState.draft,
    );
  });

  testWidgets('窄屏和放大字体下仍可到达完成按钮', (tester) async {
    tester.view.physicalSize = const Size(320, 700);
    tester.view.devicePixelRatio = 1;
    tester.binding.platformDispatcher.textScaleFactorTestValue = 1.4;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    addTearDown(
      tester.binding.platformDispatcher.clearTextScaleFactorTestValue,
    );

    await tester.pumpWidget(DiaryApp(api: MockDiaryApi()));
    await enterDiary(tester);
    await tester.enterText(find.byKey(const Key('capture-editor')), '小小一段。');
    await tester.ensureVisible(find.byKey(const Key('finish-capture')));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('finish-capture')), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}
