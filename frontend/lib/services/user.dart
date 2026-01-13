import 'package:dio/dio.dart';
import 'package:notes/constants.dart';
import 'package:notes/services/general.dart';

class UserManagementService {
  Future<void> createUser(Dio dio, String username, String password) async {
    try {
      final response = await dio.post(createUserEndpoint, data: {"username": username, "password": password});
      final genericResponse = GenericResponse.fromJson(response.data as Map<String, dynamic>);
      if (genericResponse.status != "Completed") {
        throw Exception(genericResponse.message ?? "Username might have already existed");
      }
    } catch (e) {
      rethrow;
    }
  }

  Future<bool> login(Dio dio, String username, String password) async {
    try {
      final response = await dio.post(loginEndpoint, data: {"username": username, "password": password});
      final genericResponse = GenericResponse.fromJson(response.data as Map<String, dynamic>);
      if (genericResponse.status == "Completed") {
        if (genericResponse.data != null && genericResponse.data is Map<String, dynamic>) {
          return genericResponse.data['is_login'] as bool? ?? false;
        }
        return false;
      } else {
        throw Exception(genericResponse.message ?? "Failed to login");
      }
    } catch (e) {
      rethrow;
    }
  }

  Future<Map<String, dynamic>> getUserConfigurationsSchemars(Dio dio) async {
    try {
      final response = await dio.get(getUserConfigurationsSchemarsEndpoint);
      final genericResponse = GenericResponse.fromJson(response.data as Map<String, dynamic>);
      if (genericResponse.status == "Completed") {
        return genericResponse.data as Map<String, dynamic>;
      } else {
        throw Exception(genericResponse.message ?? "Failed to get user configurations schema");
      }
    } catch (e) {
      rethrow;
    }
  }

  Future<Map<String, dynamic>> getUserConfigurationsMap(Dio dio, String username) async {
    try {
      final response = await dio.post(getUserConfigurationsEndpoint, data: {"username": username});
      final genericResponse = GenericResponse.fromJson(response.data as Map<String, dynamic>);
      if (genericResponse.status == "Completed") {
        return genericResponse.data as Map<String, dynamic>;
      } else {
        throw Exception(genericResponse.message ?? "Failed to get user configurations");
      }
    } catch (e) {
      rethrow;
    }
  }

  Future<void> updateUserConfigurationsMap(Dio dio, String username, Map<String, dynamic> config) async {
    try {
      final response = await dio.post(
        updateUserConfigurationsEndpoint,
        data: {"username": username, "user_configurations": config},
      );
      final genericResponse = GenericResponse.fromJson(response.data as Map<String, dynamic>);
      if (genericResponse.status != "Completed") {
        throw Exception(genericResponse.message ?? "Failed to update user configurations");
      }
    } catch (e) {
      rethrow;
    }
  }
}
