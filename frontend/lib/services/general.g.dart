// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'general.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

GenericResponse _$GenericResponseFromJson(Map<String, dynamic> json) =>
    GenericResponse(
      taskId: json['task_id'] as String,
      status: json['status'] as String,
      message: json['message'] as String?,
      data: json['data'],
    );

Map<String, dynamic> _$GenericResponseToJson(GenericResponse instance) =>
    <String, dynamic>{
      'task_id': instance.taskId,
      'status': instance.status,
      'message': instance.message,
      'data': instance.data,
    };
