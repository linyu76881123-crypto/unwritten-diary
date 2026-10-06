import 'package:diary_api/diary_api.dart';
import 'package:flutter/material.dart';

import 'capture_controller.dart';
import 'theme.dart';

class HomeShell extends StatefulWidget {
  const HomeShell({
    super.key,
    required this.api,
    required this.isMock,
    required this.apiOpened,
  });

  final DiaryApi api;
  final bool isMock;
  final bool apiOpened;

  @override
  State<HomeShell> createState() => _HomeShellState();
}

class _HomeShellState extends State<HomeShell> {
  late final CaptureController _capture;
  late final TextEditingController _editor;
  int _tab = 0;

  @override
  void initState() {
    super.initState();
    _capture = CaptureController(widget.api)
      ..initialize(opened: widget.apiOpened);
    _editor = TextEditingController();
  }

  @override
  void dispose() {
    _editor.dispose();
    _capture.dispose();
    super.dispose();
  }

  bool get _dark => Theme.of(context).brightness == Brightness.dark;
  Color get _card => _dark ? DiaryColors.darkPaper : DiaryColors.paper;
  Color get _border => _dark ? const Color(0xFF51423C) : DiaryColors.border;
  Color get _muted => _dark ? const Color(0xFFC1B4AA) : DiaryColors.quiet;

  @override
  Widget build(BuildContext context) => AnimatedBuilder(
    animation: _capture,
    builder: (context, _) => LayoutBuilder(
      builder: (context, size) {
        final desktop = size.maxWidth >= 900;
        return Scaffold(
          body: SafeArea(
            child: Row(
              children: [
                if (desktop) _rail(),
                Expanded(
                  child: SingleChildScrollView(
                    padding: EdgeInsets.fromLTRB(
                      desktop ? 52 : 22,
                      desktop ? 36 : 18,
                      desktop ? 52 : 22,
                      28,
                    ),
                    child: Center(
                      child: ConstrainedBox(
                        constraints: const BoxConstraints(maxWidth: 1180),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.stretch,
                          children: [
                            _header(),
                            const SizedBox(height: 35),
                            if (_tab == 0)
                              _recordContent(desktop)
                            else
                              _momentsContent(),
                          ],
                        ),
                      ),
                    ),
                  ),
                ),
              ],
            ),
          ),
          bottomNavigationBar: desktop
              ? null
              : NavigationBar(
                  selectedIndex: _tab,
                  onDestinationSelected: (value) =>
                      setState(() => _tab = value),
                  destinations: const [
                    NavigationDestination(
                      icon: Icon(Icons.edit_note_rounded),
                      selectedIcon: Icon(Icons.edit_note_rounded),
                      label: '记录',
                    ),
                    NavigationDestination(
                      icon: Icon(Icons.auto_stories_outlined),
                      selectedIcon: Icon(Icons.auto_stories_rounded),
                      label: '片段',
                    ),
                  ],
                ),
        );
      },
    ),
  );

  Widget _rail() => Container(
    decoration: BoxDecoration(
      border: Border(right: BorderSide(color: _border)),
    ),
    child: NavigationRail(
      backgroundColor: _card,
      selectedIndex: _tab,
      onDestinationSelected: (value) => setState(() => _tab = value),
      labelType: NavigationRailLabelType.all,
      groupAlignment: -0.3,
      destinations: const [
        NavigationRailDestination(
          icon: Icon(Icons.edit_note_rounded),
          label: Text('记录'),
        ),
        NavigationRailDestination(
          icon: Icon(Icons.auto_stories_outlined),
          label: Text('片段'),
        ),
      ],
    ),
  );

  Widget _header() => Row(
    children: [
      ClipRRect(
        borderRadius: BorderRadius.circular(13),
        child: Image.asset('assets/brand/app_icon.png', width: 42, height: 42),
      ),
      const SizedBox(width: 12),
      Text(
        '不写日记',
        style: Theme.of(context).textTheme.titleLarge?.copyWith(fontSize: 19),
      ),
      const Spacer(),
      if (MediaQuery.sizeOf(context).width >= 540)
        Text('留住当下，自然成篇', style: Theme.of(context).textTheme.bodyMedium),
    ],
  );

  Widget _recordContent(bool desktop) => Column(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: [
      _hero(),
      const SizedBox(height: 25),
      if (widget.isMock) ...[_mockNotice(), const SizedBox(height: 22)],
      if (desktop)
        Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Expanded(flex: 7, child: _editorCard()),
            const SizedBox(width: 25),
            Expanded(flex: 4, child: _recentCard(compact: true)),
          ],
        )
      else ...[
        _editorCard(),
        const SizedBox(height: 25),
        _recentCard(compact: false),
      ],
      const SizedBox(height: 25),
      Center(
        child: Text(
          '一小句，也值得被好好收下。',
          style: Theme.of(context).textTheme.bodyMedium,
          textAlign: TextAlign.center,
        ),
      ),
    ],
  );

  Widget _hero() {
    final now = DateTime.now();
    final date = '${now.year} 年 ${now.month} 月 ${now.day} 日';
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(date, style: Theme.of(context).textTheme.bodyMedium),
        const SizedBox(height: 11),
        Text('今天，想留住什么？', style: Theme.of(context).textTheme.displaySmall),
        const SizedBox(height: 8),
        Text(
          '不需要写得完整，从眼前这一刻开始就好。',
          style: Theme.of(context).textTheme.bodyLarge?.copyWith(color: _muted),
        ),
      ],
    );
  }

  Widget _mockNotice() => Container(
    padding: const EdgeInsets.symmetric(horizontal: 17, vertical: 13),
    decoration: BoxDecoration(
      color: _dark ? const Color(0xFF3E342F) : const Color(0xFFF3E9DF),
      borderRadius: BorderRadius.circular(15),
    ),
    child: Row(
      children: [
        const Icon(Icons.info_outline_rounded, size: 18),
        const SizedBox(width: 10),
        Expanded(
          child: Text(
            'F0 演示模式 · 内容仅保存在本次运行，关闭应用后会消失。',
            style: Theme.of(context).textTheme.bodyMedium,
          ),
        ),
      ],
    ),
  );

  Widget _editorCard() => Container(
    padding: const EdgeInsets.all(24),
    decoration: _cardDecoration(),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            _roundIcon(Icons.wb_sunny_outlined),
            const SizedBox(width: 11),
            Text('此刻的记录', style: Theme.of(context).textTheme.titleLarge),
          ],
        ),
        const SizedBox(height: 21),
        TextField(
          key: const Key('capture-editor'),
          controller: _editor,
          onChanged: _capture.updateText,
          minLines: MediaQuery.sizeOf(context).width < 600 ? 5 : 8,
          maxLines: 12,
          keyboardType: TextInputType.multiline,
          textInputAction: TextInputAction.newline,
          style: Theme.of(context).textTheme.bodyLarge?.copyWith(fontSize: 17),
          decoration: InputDecoration(
            hintText: '比如，今天有什么让你想停一停、记一记？',
            hintStyle: Theme.of(
              context,
            ).textTheme.bodyLarge?.copyWith(color: _muted),
          ),
        ),
        Divider(color: _border, height: 35),
        _saveStatus(),
        const SizedBox(height: 14),
        LayoutBuilder(
          builder: (context, box) {
            final narrow = box.maxWidth < 430;
            final actions = Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                IconButton(
                  tooltip: '添加附件将在 F1 接入',
                  onPressed: () => _futureFeature('附件导入将在 F1 接入。'),
                  icon: const Icon(Icons.attach_file_rounded),
                ),
                IconButton(
                  tooltip: '录音将在 F1 接入',
                  onPressed: () => _futureFeature('真实录音将在 F1 接入。'),
                  icon: const Icon(Icons.mic_none_rounded),
                ),
              ],
            );
            final button = FilledButton.icon(
              key: const Key('finish-capture'),
              onPressed:
                  _capture.text.trim().isEmpty ||
                      _capture.phase == SavePhase.saving
                  ? null
                  : _finish,
              icon: const Icon(Icons.arrow_forward_rounded, size: 18),
              label: const Text('收好这一刻'),
            );
            return narrow
                ? Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      Align(alignment: Alignment.centerLeft, child: actions),
                      const SizedBox(height: 9),
                      button,
                    ],
                  )
                : Row(children: [actions, const Spacer(), button]);
          },
        ),
      ],
    ),
  );

  Widget _saveStatus() {
    final (icon, label, color) = switch (_capture.phase) {
      SavePhase.ready => (Icons.circle_outlined, '写一点什么，随时可以开始。', _muted),
      SavePhase.editing => (Icons.edit_outlined, '正在写，还没有保存确认。', _muted),
      SavePhase.saving => (Icons.sync_rounded, '正在保存演示内容…', _muted),
      SavePhase.saved => (
        Icons.check_circle_outline_rounded,
        widget.isMock ? '已模拟保存 · 仅限本次运行' : '已保存',
        _dark ? const Color(0xFFD6AB91) : const Color(0xFF8A6555),
      ),
      SavePhase.failed => (
        Icons.error_outline_rounded,
        _capture.error ?? '尚未保存，请重试。',
        _dark ? const Color(0xFFEAB39E) : const Color(0xFFAE594B),
      ),
    };
    return Row(
      children: [
        Icon(icon, size: 17, color: color),
        const SizedBox(width: 8),
        Expanded(
          child: Text(label, style: TextStyle(color: color, fontSize: 13)),
        ),
        if (_capture.phase == SavePhase.failed)
          TextButton(onPressed: _capture.saveNow, child: const Text('重试')),
      ],
    );
  }

  Future<void> _finish() async {
    if (await _capture.finish() && mounted) {
      _editor.clear();
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(widget.isMock ? '已收好（演示会话）' : '已收好这一刻')),
      );
    }
  }

  void _futureFeature(String text) {
    ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(text)));
  }

  Widget _momentsContent() => Column(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: [
      Text('留下的片段', style: Theme.of(context).textTheme.displaySmall),
      const SizedBox(height: 8),
      Text('这里先展示本次演示会话里的记录。', style: Theme.of(context).textTheme.bodyLarge),
      const SizedBox(height: 25),
      if (widget.isMock) ...[_mockNotice(), const SizedBox(height: 22)],
      _recentCard(compact: false),
    ],
  );

  Widget _recentCard({required bool compact}) => Container(
    padding: const EdgeInsets.all(24),
    decoration: _cardDecoration(),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            _roundIcon(Icons.auto_stories_outlined),
            const SizedBox(width: 11),
            Expanded(
              child: Text(
                '本次的片段',
                style: Theme.of(context).textTheme.titleLarge,
              ),
            ),
            Text(
              '${_capture.recent.length}',
              style: Theme.of(context).textTheme.bodyMedium,
            ),
          ],
        ),
        const SizedBox(height: 20),
        if (_capture.loading)
          const Center(child: CircularProgressIndicator())
        else if (_capture.recent.isEmpty)
          _emptyMoments()
        else
          ..._capture.recent.take(compact ? 4 : 20).map(_momentTile),
      ],
    ),
  );

  Widget _emptyMoments() => Container(
    width: double.infinity,
    padding: const EdgeInsets.symmetric(vertical: 36, horizontal: 18),
    decoration: BoxDecoration(
      color: _dark ? const Color(0xFF39312D) : const Color(0xFFFAF3EC),
      borderRadius: BorderRadius.circular(20),
    ),
    child: Column(
      children: [
        Icon(
          Icons.spa_outlined,
          size: 40,
          color: _dark ? const Color(0xFFE7B4A0) : DiaryColors.peach,
        ),
        const SizedBox(height: 13),
        Text('这里还空着，刚刚好。', style: Theme.of(context).textTheme.bodyLarge),
        const SizedBox(height: 5),
        Text('第一段话写下后，就会出现在这里。', style: Theme.of(context).textTheme.bodyMedium),
      ],
    ),
  );

  Widget _momentTile(Capture capture) {
    final time = capture.occurredAt.toLocal();
    final timeLabel =
        '${time.hour.toString().padLeft(2, '0')}:${time.minute.toString().padLeft(2, '0')}';
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Container(
        padding: const EdgeInsets.all(17),
        decoration: BoxDecoration(
          color: _dark ? const Color(0xFF39312D) : const Color(0xFFFAF6F1),
          borderRadius: BorderRadius.circular(18),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Text(timeLabel, style: Theme.of(context).textTheme.bodyMedium),
                const Spacer(),
                Text(
                  capture.state == CaptureState.committed ? '已收好' : '草稿',
                  style: Theme.of(
                    context,
                  ).textTheme.bodyMedium?.copyWith(fontSize: 12),
                ),
              ],
            ),
            const SizedBox(height: 8),
            Text(
              capture.draftText,
              maxLines: 4,
              overflow: TextOverflow.ellipsis,
              style: Theme.of(context).textTheme.bodyLarge,
            ),
          ],
        ),
      ),
    );
  }

  Widget _roundIcon(IconData icon) => Container(
    width: 38,
    height: 38,
    decoration: BoxDecoration(
      color: _dark ? const Color(0xFF594137) : const Color(0xFFF8E8DB),
      borderRadius: BorderRadius.circular(13),
    ),
    child: Icon(
      icon,
      size: 20,
      color: _dark ? const Color(0xFFF2C0A9) : DiaryColors.peach,
    ),
  );

  BoxDecoration _cardDecoration() => BoxDecoration(
    color: _card,
    borderRadius: BorderRadius.circular(26),
    border: Border.all(color: _border),
    boxShadow: _dark
        ? null
        : const [
            BoxShadow(
              color: Color(0x0D725B4F),
              blurRadius: 26,
              offset: Offset(0, 12),
            ),
          ],
  );
}
