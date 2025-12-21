import 'package:dio/dio.dart';
import 'package:json_annotation/json_annotation.dart';
import 'package:notes/constants.dart';
import 'package:notes/services/general.dart';

part 'user.g.dart';

@JsonSerializable(fieldRename: FieldRename.snake)
class SearchConfiguration {
  int documentChunkSize;

  SearchConfiguration({required this.documentChunkSize});

  factory SearchConfiguration.fromJson(Map<String, dynamic> json) =>
      _$SearchConfigurationFromJson(json);
  Map<String, dynamic> toJson() => _$SearchConfigurationToJson(this);
}

@JsonSerializable(fieldRename: FieldRename.snake)
class UserConfigurations {
  SearchConfiguration search;

  UserConfigurations({required this.search});

  factory UserConfigurations.fromJson(Map<String, dynamic> json) =>
      _$UserConfigurationsFromJson(json);
  Map<String, dynamic> toJson() => _$UserConfigurationsToJson(this);
}

class UserManagementService {
  Future<void> createUser(Dio dio, String username, String password) async {
    try {
      final response = await dio.post(
        createUserEndpoint,
        data: {
          "username": username,
          "password": password,
        },
      );
      final genericResponse =
          GenericResponse.fromJson(response.data as Map<String, dynamic>);
      if (genericResponse.status != "Completed") {
        throw Exception(genericResponse.message ??
            "Username might have already existed");
      }
    } catch (e) {
      rethrow;
    }
  }

  Future<bool> login(Dio dio, String username, String password) async {
    try {
      final response = await dio.post(
        loginEndpoint,
        data: {
          "username": username,
          "password": password,
        },
      );
      final genericResponse =
          GenericResponse.fromJson(response.data as Map<String, dynamic>);
      if (genericResponse.status == "Completed") {
        if (genericResponse.data != null &&
            genericResponse.data is Map<String, dynamic>) {
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

  Future<UserConfigurations> getUserConfigurations(
      Dio dio, String username) async {
    try {
      final response = await dio.post(
        getUserConfigurationsEndpoint,
        data: {"username": username},
      );
      final genericResponse =
          GenericResponse.fromJson(response.data as Map<String, dynamic>);
      if (genericResponse.status == "Completed") {
        return UserConfigurations.fromJson(
            genericResponse.data as Map<String, dynamic>);
      } else {
        throw Exception(
            genericResponse.message ?? "Failed to get user configurations");
      }
    } catch (e) {
      rethrow;
    }
  }

  Future<void> updateUserConfigurations(
      Dio dio, String username, UserConfigurations config) async {
    try {
      final response = await dio.post(
        updateUserConfigurationsEndpoint,
        data: {
          "username": username,
          "user_configurations": config.toJson(),
        },
      );
      final genericResponse =
          GenericResponse.fromJson(response.data as Map<String, dynamic>);
      if (genericResponse.status != "Completed") {
        throw Exception(
            genericResponse.message ?? "Failed to update user configurations");
      }
    } catch (e) {
      rethrow;
    }
  }
}
