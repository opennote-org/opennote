import 'package:flutter/foundation.dart';
import 'package:json_annotation/json_annotation.dart';
import 'package:notes/json_serialization.dart';
import 'package:notes/state/services.dart';
import 'package:uuid/uuid.dart';

part 'users.g.dart';

@JsonSerializable()
class LocalUserLoginRecord with JsonSerializationMixin {
  bool hasUserLoggedIn;
  String username;
  String verificationHash = "xxhAzB0";
  DateTime lastLogin = DateTime.now();

  LocalUserLoginRecord(
    this.hasUserLoggedIn,
    this.username,
    this.verificationHash,
  );

  factory LocalUserLoginRecord.fromJson(Map<String, dynamic> json) =>
      _$LocalUserLoginRecordFromJson(json);

  Map<String, dynamic> toJson() => _$LocalUserLoginRecordToJson(this);
}

mixin Users on ChangeNotifier, Services {
  String? username;

  void registerLocalLoginStatus(String username, bool hasLoggedIn) {
    // We use a uuid as temporary filler.
    // TODO: have a secure authentication for users.
    final uuid = Uuid();
    setDataToLocalStorage(
      'login_status',
      'system',
      LocalUserLoginRecord(hasLoggedIn, username, uuid.v4()).toString(),
    );
  }

  Future<String?> hasUserLoggedIn() async {
    final String? content = await readDataFromLocalStorage(
      'login_status',
      'system',
    );
    if (content != null) {
      final record = JsonSerializationMixin.fromString(
        content,
        LocalUserLoginRecord.fromJson,
      );

      if (DateTime.now().difference(record.lastLogin).inDays >= 30) {
        // Remove stale entry
        await removeDataFromLocalStorage('login_status', 'system');
        return null;
      }

      return record.username;
    }
    return null;
  }

  Future<void> logout() async {
    final String? content = await readDataFromLocalStorage(
      'login_status',
      'system',
    );
    if (content != null) {
      await removeDataFromLocalStorage('login_status', 'system');
    }
    username = null;
    notifyListeners();
  }
}
