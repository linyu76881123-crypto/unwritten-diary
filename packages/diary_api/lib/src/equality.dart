/// 包内使用的集合相等判断。不从 `diary_api.dart` 导出，属于实现细节。
library;

bool listEquals<T>(List<T>? a, List<T>? b) {
  if (identical(a, b)) {
    return true;
  }
  if (a == null || b == null || a.length != b.length) {
    return false;
  }
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) {
      return false;
    }
  }
  return true;
}

bool setEquals<T>(Set<T>? a, Set<T>? b) {
  if (identical(a, b)) {
    return true;
  }
  if (a == null || b == null || a.length != b.length) {
    return false;
  }
  return a.containsAll(b);
}

bool mapEquals<K, V>(Map<K, V>? a, Map<K, V>? b) {
  if (identical(a, b)) {
    return true;
  }
  if (a == null || b == null || a.length != b.length) {
    return false;
  }
  for (final entry in a.entries) {
    if (!b.containsKey(entry.key) || b[entry.key] != entry.value) {
      return false;
    }
  }
  return true;
}

/// 对无序集合求哈希。`Set` 的迭代顺序不保证稳定，直接 `Object.hashAll`
/// 会让两个相等的对象得到不同的 hashCode。
int unorderedHash(Iterable<Object?> items) {
  final values = items.map((item) => item.toString()).toList()..sort();
  return Object.hashAll(values);
}