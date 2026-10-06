# 不写日记 · F0 Flutter 客户端

首版目标平台为 Windows x64 与 Android arm64。本目录是 F0 可运行应用壳：先展示中文封面，点击「开始记录」时由手绘小猫加载动效陪伴初始化，然后进入记录页。应用还包含自适应布局、深浅色主题、输入与明确的 Mock 保存状态。真实录音、文件导入和 Rust 资料库接线分别在 F1、B1 桥接及 F2 完成。

## 运行

在仓库根目录安装 Flutter SDK 后：

```bash
cd apps/diary_app
flutter pub get
flutter run -d windows
# 已配置 Android SDK 和设备时：flutter run -d <device-id>
```

`DiaryApp(api: ...)` 注入 `DiaryApi`。目前 `main.dart` 注入 `MockDiaryApi`；未来真实桥接适配实现同一接口后，只替换这个注入点。页面组件不直接访问数据库、Rust 内部对象或远端模型服务。

## F0 的真实范围

- 支持输入中文长文本；输入法 Enter 是换行/候选确认，提交只由明确按钮触发。
- 文字修改后自动走 `createDraft` / `saveDraft`；保存失败保留输入并可重试；点「收好这一刻」走 `commit`。
- 本次会话的记录在「片段」里可见。Mock 完全在内存中，关闭应用后消失；所有保存提示与页头都明确标记为演示。
- 录音与附件入口目前只说明 F1 接入状态，不制造假录音或假导入结果。
- Android、Windows 使用同一语义布局；窄屏用底部导航，宽屏用侧边导航。
- 封面与应用图标使用同一套暖杏、陶土色落日渐变和抽象线条；加载时由 Flutter 逐笔绘出光晕与地平线。若系统要求减少动画，画面保持静态。初始化失败时停留在封面并可重试。

## 检查

```bash
cd packages/diary_mock && dart analyze && dart test
cd ../../apps/diary_app && flutter analyze && flutter test
```

当前 Mock 不能用于验证断电/重启持久化、Android 真机录音或真实核心联调。相关验收留给 F1、F2 与桥接任务，并按仓库验收计划执行。
