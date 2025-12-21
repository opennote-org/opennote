// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'health.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

BackendHealthStatus _$BackendHealthStatusFromJson(Map<String, dynamic> json) =>
    BackendHealthStatus(
      status: json['status'] as String,
      timestamp: json['timestamp'] as String,
    );

Map<String, dynamic> _$BackendHealthStatusToJson(
  BackendHealthStatus instance,
) => <String, dynamic>{
  'status': instance.status,
  'timestamp': instance.timestamp,
};
