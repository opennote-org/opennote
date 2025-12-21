import 'package:dio/dio.dart';
import 'package:json_annotation/json_annotation.dart';
import 'package:notes/constants.dart';

part 'general.g.dart';

@JsonSerializable(fieldRename: FieldRename.snake)
class GenericResponse {
  String taskId;
  String status;
  String? message;
  dynamic data;

  GenericResponse({
    required this.taskId,
    required this.status,
    this.message,
    this.data,
  });

  factory GenericResponse.fromJson(Map<String, dynamic> json) =>
      _$GenericResponseFromJson(json);

  Map<String, dynamic> toJson() => _$GenericResponseToJson(this);
}

class GeneralService {
  Future<GenericResponse> retrieveTaskResult(Dio dio, String taskId) async {
    final response = await dio.post(
      retrieveTaskResultEndpoint,
      data: {"task_id": taskId},
    );
    return GenericResponse.fromJson(response.data as Map<String, dynamic>);
  }
}