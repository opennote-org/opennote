import 'dart:convert';

mixin JsonSerializationMixin {
  Map<String, dynamic> toJson();

  static T fromString<T>(String string, T Function(Map<String, dynamic>) fromJson) {
    final json = jsonDecode(string);
    return fromJson(json);
  }

  @override
  String toString() {
    final json = toJson();
    return jsonEncode(json);
  }
}