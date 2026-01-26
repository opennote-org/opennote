// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'users.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

LocalUserLoginRecord _$LocalUserLoginRecordFromJson(
  Map<String, dynamic> json,
) => LocalUserLoginRecord(
  json['hasUserLoggedIn'] as bool,
  json['username'] as String,
  json['verificationHash'] as String,
)..lastLogin = DateTime.parse(json['lastLogin'] as String);

Map<String, dynamic> _$LocalUserLoginRecordToJson(
  LocalUserLoginRecord instance,
) => <String, dynamic>{
  'hasUserLoggedIn': instance.hasUserLoggedIn,
  'username': instance.username,
  'verificationHash': instance.verificationHash,
  'lastLogin': instance.lastLogin.toIso8601String(),
};
