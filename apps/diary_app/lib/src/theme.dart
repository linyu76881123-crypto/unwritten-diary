import 'package:flutter/material.dart';

abstract final class DiaryColors {
  static const ink = Color(0xFF4D403A);
  static const quiet = Color(0xFF8B776D);
  static const peach = Color(0xFFAA6A52);
  static const paper = Color(0xFFFFFCF8);
  static const canvas = Color(0xFFFAF5EF);
  static const border = Color(0xFFEDDFD3);
  static const darkInk = Color(0xFFF6EEE8);
  static const darkCanvas = Color(0xFF211E1D);
  static const darkPaper = Color(0xFF302A28);
}

abstract final class DiaryTheme {
  static ThemeData get light => _build(Brightness.light);
  static ThemeData get dark => _build(Brightness.dark);

  static ThemeData _build(Brightness brightness) {
    final dark = brightness == Brightness.dark;
    final scheme = ColorScheme.fromSeed(
      seedColor: DiaryColors.peach,
      brightness: brightness,
      surface: dark ? DiaryColors.darkPaper : DiaryColors.paper,
    );
    return ThemeData(
      useMaterial3: true,
      brightness: brightness,
      colorScheme: scheme,
      scaffoldBackgroundColor: dark
          ? DiaryColors.darkCanvas
          : DiaryColors.canvas,
      fontFamily: 'Microsoft YaHei',
      fontFamilyFallback: const ['Microsoft YaHei', 'Noto Sans CJK SC'],
      textTheme: TextTheme(
        displaySmall: TextStyle(
          fontSize: 34,
          fontWeight: FontWeight.w700,
          height: 1.32,
          letterSpacing: -1.0,
          color: dark ? DiaryColors.darkInk : DiaryColors.ink,
        ),
        titleLarge: TextStyle(
          fontSize: 22,
          fontWeight: FontWeight.w700,
          color: dark ? DiaryColors.darkInk : DiaryColors.ink,
        ),
        bodyLarge: TextStyle(
          fontSize: 16,
          height: 1.55,
          color: dark ? DiaryColors.darkInk : DiaryColors.ink,
        ),
        bodyMedium: TextStyle(
          fontSize: 14,
          height: 1.5,
          color: dark ? const Color(0xFFC5B9B2) : DiaryColors.quiet,
        ),
      ),
      filledButtonTheme: FilledButtonThemeData(
        style: FilledButton.styleFrom(
          backgroundColor: dark ? const Color(0xFFE3A38C) : DiaryColors.peach,
          foregroundColor: dark ? DiaryColors.darkCanvas : Colors.white,
          padding: const EdgeInsets.symmetric(horizontal: 23, vertical: 18),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(14),
          ),
          textStyle: const TextStyle(
            fontFamily: 'Microsoft YaHei',
            fontSize: 15,
            fontWeight: FontWeight.w700,
          ),
        ),
      ),
      navigationBarTheme: NavigationBarThemeData(
        backgroundColor: dark ? DiaryColors.darkPaper : DiaryColors.paper,
        indicatorColor: dark
            ? const Color(0xFF5A3A31)
            : const Color(0xFFF3DECF),
        labelTextStyle: WidgetStateProperty.all(const TextStyle(fontSize: 12)),
      ),
      inputDecorationTheme: const InputDecorationTheme(
        border: InputBorder.none,
      ),
    );
  }
}
