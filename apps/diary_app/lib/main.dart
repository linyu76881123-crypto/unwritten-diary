import 'package:diary_api/diary_api.dart';
import 'package:diary_mock/diary_mock.dart';
import 'package:flutter/material.dart';

import 'src/intro_screen.dart';
import 'src/theme.dart';

void main() => runApp(DiaryApp(api: MockDiaryApi()));

/// 通过 DiaryApi 注入数据源；真实桥接完成后页面不需改动。
class DiaryApp extends StatelessWidget {
  const DiaryApp({super.key, required this.api});

  final DiaryApi api;

  @override
  Widget build(BuildContext context) => MaterialApp(
    title: '不写日记',
    debugShowCheckedModeBanner: false,
    theme: DiaryTheme.light,
    darkTheme: DiaryTheme.dark,
    themeMode: ThemeMode.system,
    home: IntroScreen(api: api, isMock: api is MockDiaryApi),
  );
}
