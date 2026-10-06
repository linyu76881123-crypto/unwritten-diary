import 'package:flutter/material.dart';

/// 渐变和不闭合线条组成的落日意象，不使用具象插画。
class SunsetArtwork extends StatelessWidget {
  const SunsetArtwork({super.key});

  @override
  Widget build(BuildContext context) => Semantics(
    label: '朦胧的暖色落日与层叠的细线',
    image: true,
    child: const CustomPaint(painter: _SunsetArtworkPainter()),
  );
}

class _SunsetArtworkPainter extends CustomPainter {
  const _SunsetArtworkPainter();

  @override
  void paint(Canvas canvas, Size size) {
    canvas.save();
    canvas.scale(size.width / 560, size.height / 480);

    final glow = Paint()
      ..shader = const RadialGradient(
        colors: [Color(0x88E6B18F), Color(0x42F0CFB5), Color(0x00FFF5E9)],
        stops: [0, 0.52, 1],
      ).createShader(const Rect.fromLTWH(72, 24, 430, 430));
    canvas.drawOval(const Rect.fromLTWH(72, 24, 430, 430), glow);

    // 两个半透明色层让太阳像暮光中的色块，不画成实体圆球。
    final sun = Paint()
      ..shader =
          const RadialGradient(
            center: Alignment(-0.26, -0.36),
            radius: 1.05,
            colors: [Color(0xBCEFC79F), Color(0x96D99675), Color(0x35E2B69A)],
            stops: [0, 0.62, 1],
          ).createShader(
            Rect.fromCircle(center: const Offset(292, 223), radius: 118),
          );
    canvas.drawCircle(const Offset(292, 223), 118, sun);

    final veil = Paint()
      ..color = const Color(0x55FFF8EF)
      ..maskFilter = const MaskFilter.blur(BlurStyle.normal, 24);
    canvas.drawOval(const Rect.fromLTWH(112, 279, 352, 54), veil);

    final fine = Paint()
      ..color = const Color(0x9A936B5B)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.1
      ..strokeCap = StrokeCap.round;
    final quiet = Paint()
      ..color = const Color(0x668E6A5B)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 0.75
      ..strokeCap = StrokeCap.round;

    // 不完整的弧线像远处的光晕，也像翻页留下的线。
    canvas.drawArc(
      const Rect.fromLTWH(94, 24, 394, 394),
      3.36,
      3.84,
      false,
      fine,
    );
    canvas.drawArc(
      const Rect.fromLTWH(125, 55, 332, 332),
      3.58,
      3.27,
      false,
      quiet,
    );
    canvas.drawArc(
      const Rect.fromLTWH(154, 84, 274, 274),
      3.68,
      2.62,
      false,
      quiet,
    );

    Path horizon(double y, double lift) => Path()
      ..moveTo(24, y)
      ..cubicTo(145, y - lift, 222, y + lift * 0.5, 310, y)
      ..cubicTo(407, y - lift, 481, y + lift * 0.2, 548, y - 2);
    canvas.drawPath(horizon(305, 6), fine);
    canvas.drawPath(horizon(328, 8), quiet);
    canvas.drawPath(horizon(359, 12), quiet);
    canvas.drawPath(horizon(401, 17), quiet);

    final accent = Paint()
      ..color = const Color(0x88936857)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.35
      ..strokeCap = StrokeCap.round;
    canvas.drawLine(const Offset(410, 188), const Offset(461, 188), accent);
    canvas.drawLine(const Offset(418, 195), const Offset(449, 195), quiet);
    canvas.restore();
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}

/// 用同一组线条缓慢画出一轮落日；尊重系统的减少动画设置。
class SunsetLoading extends StatefulWidget {
  const SunsetLoading({super.key});

  @override
  State<SunsetLoading> createState() => _SunsetLoadingState();
}

class _SunsetLoadingState extends State<SunsetLoading>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 1800),
  );

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (MediaQuery.of(context).disableAnimations) {
      _controller.stop();
      _controller.value = 0.78;
    } else if (!_controller.isAnimating) {
      _controller.repeat();
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Semantics(
    label: '落日的线条正在缓缓绘出',
    child: AnimatedBuilder(
      animation: _controller,
      builder: (context, _) => CustomPaint(
        size: const Size(250, 220),
        painter: _SunsetLoadingPainter(_controller.value),
      ),
    ),
  );
}

class _SunsetLoadingPainter extends CustomPainter {
  const _SunsetLoadingPainter(this.progress);

  final double progress;

  @override
  void paint(Canvas canvas, Size size) {
    canvas.save();
    canvas.scale(size.width / 250, size.height / 220);
    final glow = Paint()
      ..shader = const RadialGradient(
        colors: [Color(0x7BE4B18E), Color(0x26F0D1BB), Color(0x00FFF4E8)],
      ).createShader(const Rect.fromLTWH(31, 11, 190, 190));
    canvas.drawCircle(const Offset(126, 105), 95, glow);

    final line = Paint()
      ..color = const Color(0xFF936A58)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.8
      ..strokeCap = StrokeCap.round;
    final faint = Paint()
      ..color = const Color(0xA39C7562)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.1
      ..strokeCap = StrokeCap.round;

    final arc = Path()
      ..addArc(const Rect.fromLTWH(64, 40, 124, 124), 2.94, 4.48);
    _drawPart(canvas, arc, progress, line);
    final inner = Path()
      ..addArc(const Rect.fromLTWH(79, 55, 94, 94), 3.38, 3.42);
    _drawPart(canvas, inner, (progress - 0.2) / 0.75, faint);

    final horizon = Path()
      ..moveTo(23, 151)
      ..cubicTo(77, 147, 111, 154, 147, 151)
      ..cubicTo(190, 147, 217, 151, 230, 149);
    _drawPart(canvas, horizon, (progress - 0.12) / 0.78, line);
    final lower = Path()
      ..moveTo(62, 171)
      ..cubicTo(116, 167, 157, 175, 204, 169);
    _drawPart(canvas, lower, (progress - 0.43) / 0.52, faint);
    canvas.restore();
  }

  void _drawPart(Canvas canvas, Path path, double amount, Paint paint) {
    final t = amount.clamp(0.0, 1.0);
    if (t == 0) return;
    for (final metric in path.computeMetrics()) {
      canvas.drawPath(metric.extractPath(0, metric.length * t), paint);
    }
  }

  @override
  bool shouldRepaint(covariant _SunsetLoadingPainter oldDelegate) =>
      oldDelegate.progress != progress;
}
