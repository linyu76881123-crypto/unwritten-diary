# diary_api

「不写日记」两层共享的 Dart 类型包，依据 [`docs/development/01-共享接口契约.md`](../../docs/development/01-共享接口契约.md)（v1 已冻结）。

页面只依赖这里的抽象接口。实际产品用桥接层的实现，页面测试用 `packages/diary_mock` 的模拟实现，两者必须实现同一组方法、错误与事件语义。

## 内容

| 文件 | 内容 |
|---|---|
| `lib/src/values.dart` | 枚举（带稳定 `wire` 值）、契约第 7 节的错误码、`DiaryException` |
| `lib/src/models_content.dart` | 契约 2.1–2.4：Capture、SourceItem、SourceRevision、Asset、ExtractedContent、SourceLocator |
| `lib/src/models_diary.dart` | 契约 2.5–2.7：DiaryDay、DiaryVersion、StyleProfile、SearchRequest/Hit/Snapshot、Job、Provider、Plugin |
| `lib/src/models_transport.dart` | 契约 1.3、4 节：CoreInfo、分页结果、票据与报告 |
| `lib/src/events.dart` | 契约第 6 节的持久业务事件 |
| `lib/src/diary_api.dart` | `DiaryApi` 抽象接口，契约第 4 节全部方法 |
| `lib/src/platform_host.dart` | `PlatformHost` 抽象接口，契约第 5 节 |

## 刻意的设计选择

- **不依赖 Flutter。** 核心层与 Mock 一致性测试都应该能在纯 Dart 环境跑，不需要 Flutter SDK。CI 用 `dart` 而不是 `flutter`。
- **不做 JSON 序列化。** 传输编码由桥接层负责。这里只放不可变值对象，避免三套序列化代码互相漂移。
- **枚举带 `wire` 值。** Dart 侧名字可以改，`wire` 值与契约、持久层、桥接层保持一致。
- **时间用 `DateTime` + 独立时区字段。** 不把时区塞进时间戳；`dayKey` 与 `timeZone` 分开存，改时区不搬动历史。
- **毫秒用 `int`。** 与契约里的 `startMs` / `durationMs` 直接对应，避免桥接层来回换算。
- **ID 用普通 `String`。** 契约说 ID 是不透明字符串，调用方不得解析或拼接；不引入额外的 ID 包装类型增加桥接摩擦。

## 使用

```yaml
dependencies:
  diary_api:
    path: ../diary_api
```

```dart
import 'package:diary_api/diary_api.dart';

class MyPage {
  MyPage(this._api);
  final DiaryApi _api;
}
```

## 校验

```bash
dart pub get
dart analyze --fatal-infos
dart test
```

测试做两件事：

1. **值对象语义**：枚举 `wire` 往返、契约第 7 节错误码齐全、`copyWith` 与相等判断（含集合与 map 字段）。
2. **契约一致性**：读取 [`tests/fixtures/scenarios/m0-scenarios.json`](../../tests/fixtures/scenarios/m0-scenarios.json)，验证
   - 场景文件里的 `contractVersion` 与包内常量一致；
   - 每条场景都有必备字段、`id` 唯一、验收编号格式正确；
   - 场景步骤只使用契约里列出的方法名；
   - 契约方法在 `DiaryApi` 上都有对应实现；
   - `DiaryApi` 上没有契约之外的多余方法（接口漂移会被这条挡住）。

契约方法名与 Dart 方法名的对应关系写在 `test/contract_surface_test.dart` 的 `kContractToDartMethod` 里，改动任何一侧都要同步更新。

## 尚未完成

- 没有 Mock 实现（属 `packages/diary_mock`，前端层）。
- 没有桥接生成（属 `packages/diary_bridge` / `crates/diary_bridge`）。
- 契约 1.4 节的七个未决实现选择仍待 M0 实测决定，它们不影响本包的接口形状。