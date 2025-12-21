import 'dart:convert';

import 'package:dio/dio.dart';
import 'package:json_annotation/json_annotation.dart';
import 'package:notes/constants.dart';

part 'health.g.dart';

@JsonSerializable()
class BackendHealthStatus {
  String status;
  String timestamp;

  BackendHealthStatus({required this.status, required this.timestamp});
}

class BackendHealthCheckService {
  Future<BackendHealthStatus> getBackendHealthStatus(Dio dio) async {
    final Response response = await dio.get(backendHealthCheckEndpoint);
    return _$BackendHealthStatusFromJson(response.data!);
  }
}
