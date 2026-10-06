import 'dart:async';
import 'dart:math' as math;

import 'package:diary_api/diary_api.dart';
import 'package:flutter/material.dart';

import 'home_shell.dart';
import 'sunset_art.dart';

/// 一张可停留的中文封面；开始记录时展示短暂的手绘加载动效。
class IntroScreen extends StatefulWidget {
  const IntroScreen({super.key, required this.api, required this.isMock});

  final DiaryApi api;
  final bool isMock;

  @override
  State<IntroScreen> createState() => _IntroScreenState();
}

class _IntroScreenState extends State<IntroScreen> {
  bool _loading = false;
  bool _entered = false;
  String? _error;

  Future<void> _begin() async {
    if (_loading) return;
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      // 真实的初始化与一段短动画并行，不把动画时间叠加在初始化之后。
      await Future.wait<void>([
        () async {
          await widget.api.open();
        }(),
        Future<void>.delayed(const Duration(milliseconds: 1350)),
      ]);
      if (mounted) setState(() => _entered = true);
    } catch (_) {
      if (mounted) {
        setState(() {
          _loading = false;
          _error = '暂时没能打开，点一下再试。';
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_entered) {
      return HomeShell(api: widget.api, isMock: widget.isMock, apiOpened: true);
    }
    return Scaffold(
      backgroundColor: const Color(0xFFFAF2E8),
      body: AnimatedSwitcher(
        duration: const Duration(milliseconds: 320),
        child: _loading ? _loadingView() : _coverView(),
      ),
    );
  }

  Widget _coverView() => Container(
    key: const ValueKey('cover'),
    decoration: const BoxDecoration(
      gradient: LinearGradient(
        begin: Alignment.topLeft,
        end: Alignment.bottomRight,
        colors: [Color(0xFFFFF9F2), Color(0xFFEBCAB6), Color(0xFFFAEFE4)],
        stops: [0, 0.52, 1],
      ),
    ),
    child: SafeArea(
      child: LayoutBuilder(
        builder: (context, bounds) {
          final wide = bounds.maxWidth >= 780;
          return Center(
            child: SingleChildScrollView(
              padding: EdgeInsets.symmetric(
                horizontal: wide ? 52 : 27,
                vertical: wide ? 34 : 24,
              ),
              child: ConstrainedBox(
                constraints: const BoxConstraints(maxWidth: 1080),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    _brand(),
                    SizedBox(height: wide ? 45 : 16),
                    if (wide)
                      Row(
                        children: [
                          Expanded(flex: 11, child: _art(510)),
                          const SizedBox(width: 52),
                          Expanded(flex: 9, child: _copy(wide: true)),
                        ],
                      )
                    else ...[
                      _art(math.min(360, bounds.maxHeight * 0.43)),
                      const SizedBox(height: 9),
                      _copy(wide: false),
                    ],
                    SizedBox(height: wide ? 34 : 28),
                    Text(
                      '一个为片刻留位置的地方',
                      textAlign: wide ? TextAlign.left : TextAlign.center,
                      style: const TextStyle(
                        fontSize: 12,
                        letterSpacing: 2,
                        color: Color(0xFF947A6C),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          );
        },
      ),
    ),
  );

  Widget _brand() => Row(
    children: [
      ClipRRect(
        borderRadius: BorderRadius.circular(11),
        child: Image.asset('assets/brand/app_icon.png', width: 36, height: 36),
      ),
      const SizedBox(width: 11),
      const Text(
        '不写日记',
        style: TextStyle(
          fontSize: 17,
          fontWeight: FontWeight.w700,
          letterSpacing: 1,
          color: Color(0xFF4D403A),
        ),
      ),
      const Spacer(),
      if (MediaQuery.sizeOf(context).width >= 400)
        const Text(
          '留给自己的小天地',
          style: TextStyle(fontSize: 11, color: Color(0xFF947A6C)),
        ),
    ],
  );

  Widget _art(double height) =>
      SizedBox(height: height, child: const SunsetArtwork());

  Widget _copy({required bool wide}) => Column(
    crossAxisAlignment: wide
        ? CrossAxisAlignment.start
        : CrossAxisAlignment.center,
    children: [
      Text(
        '给每个转瞬即逝的片刻',
        textAlign: wide ? TextAlign.left : TextAlign.center,
        style: TextStyle(
          fontSize: 12,
          letterSpacing: 2,
          color: Color(0xFFAD765E),
          fontWeight: FontWeight.w600,
        ),
      ),
      const SizedBox(height: 14),
      Text(
        '把今天，\n轻轻收好。',
        textAlign: wide ? TextAlign.left : TextAlign.center,
        style: TextStyle(
          fontSize: wide ? 52 : 39,
          height: 1.32,
          fontWeight: FontWeight.w500,
          fontFamily: 'Noto Serif CJK SC',
          fontFamilyFallback: const ['Noto Serif SC', 'serif'],
          letterSpacing: -0.5,
          color: const Color(0xFF4D403A),
        ),
      ),
      const SizedBox(height: 14),
      Text(
        '不用写成完整的日记。\n从一句话、一个念头开始就好。',
        textAlign: wide ? TextAlign.left : TextAlign.center,
        style: TextStyle(height: 1.7, fontSize: 15, color: Color(0xFF846F64)),
      ),
      const SizedBox(height: 27),
      ConstrainedBox(
        constraints: BoxConstraints(minWidth: wide ? 220 : 240),
        child: FilledButton(
          key: const Key('enter-diary'),
          onPressed: _begin,
          child: const Text('开始记录  →'),
        ),
      ),
      if (_error != null) ...[
        const SizedBox(height: 12),
        Text(_error!, style: const TextStyle(color: Color(0xFF9A5949))),
      ],
      if (widget.isMock) ...[
        const SizedBox(height: 14),
        const Text(
          'F0 演示版 · 内容关闭后会消失',
          style: TextStyle(fontSize: 11, color: Color(0xFF947E72)),
        ),
      ],
    ],
  );

  Widget _loadingView() => Container(
    key: const ValueKey('loading'),
    decoration: const BoxDecoration(
      gradient: LinearGradient(
        begin: Alignment.topCenter,
        end: Alignment.bottomCenter,
        colors: [Color(0xFFFFF8F0), Color(0xFFECCCB9), Color(0xFFF9EDE2)],
      ),
    ),
    child: SafeArea(
      child: Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const SunsetLoading(),
            const SizedBox(height: 26),
            const Text(
              '正在收拢今天的光',
              style: TextStyle(
                fontSize: 23,
                fontWeight: FontWeight.w500,
                fontFamily: 'Noto Serif CJK SC',
                fontFamilyFallback: ['Noto Serif SC', 'serif'],
                color: Color(0xFF4D403A),
              ),
            ),
            const SizedBox(height: 8),
            const Text(
              '稍等一下，就可以开始记录了。',
              style: TextStyle(fontSize: 13, color: Color(0xFF846F64)),
            ),
          ],
        ),
      ),
    ),
  );
}
