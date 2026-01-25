import 'package:dio/dio.dart';
import 'package:notes/constants.dart';
import 'package:notes/services/general.dart';
import 'package:uuid/uuid.dart';

enum BackupScope { user }

class BackupScopeIndicator {
  final BackupScope scope;
  final String id;
  final String backupId;

  BackupScopeIndicator({
    required this.scope,
    required this.id,
    required this.backupId,
  });

  dynamic toJson() {
    return '"${scope.name}"/$id/$backupId';
  }

  factory BackupScopeIndicator.fromJson(dynamic json) {
    if (json is String) {
      final parts = json.split('/');
      if (parts.length != 3)
        throw Exception("Invalid BackupScopeIndicator format");

      String scopeStr = parts[0];
      if (scopeStr.startsWith('"') && scopeStr.endsWith('"')) {
        scopeStr = scopeStr.substring(1, scopeStr.length - 1);
      }

      return BackupScopeIndicator(
        scope: BackupScope.values.firstWhere((e) => e.name == scopeStr),
        id: parts[1],
        backupId: parts[2],
      );
    }
    throw Exception("Invalid type for BackupScopeIndicator");
  }
}

class BackupListItem {
  final String id;
  final String createdAt;
  final BackupScopeIndicator scope;

  BackupListItem({
    required this.id,
    required this.createdAt,
    required this.scope,
  });

  factory BackupListItem.fromJson(Map<String, dynamic> json) {
    return BackupListItem(
      id: json['id'] as String,
      createdAt: json['created_at'] as String,
      scope: BackupScopeIndicator.fromJson(json['scope']),
    );
  }
}

class BackupService {
  Future<List<BackupListItem>> getBackupsList(Dio dio, String username) async {
    try {
      // Pass empty string for backupId when listing
      final scope = BackupScopeIndicator(
        scope: BackupScope.user,
        id: username,
        backupId: "",
      );
      final response = await dio.post(
        getBackupsListEndpoint,
        data: {"scope": scope.toJson()},
      );
      final genericResponse = GenericResponse.fromJson(
        response.data as Map<String, dynamic>,
      );

      if (genericResponse.status == "Completed") {
        if (genericResponse.data != null &&
            genericResponse.data['backups'] != null) {
          final list = genericResponse.data['backups'] as List;
          return list
              .map((e) => BackupListItem.fromJson(e as Map<String, dynamic>))
              .toList();
        }
        return [];
      } else {
        throw Exception(
          genericResponse.message ?? "Failed to get backups list",
        );
      }
    } catch (e) {
      rethrow;
    }
  }

  Future<void> removeBackups(Dio dio, List<String> backupIds) async {
    try {
      final response = await dio.post(
        removeBackupsEndpoint,
        data: {"backup_ids": backupIds},
      );
      final genericResponse = GenericResponse.fromJson(
        response.data as Map<String, dynamic>,
      );

      if (genericResponse.status != "Completed") {
        throw Exception(genericResponse.message ?? "Failed to remove backups");
      }
    } catch (e) {
      rethrow;
    }
  }

  /// Returns task_id
  Future<String> backup(Dio dio, String username) async {
    try {
      final backupId = const Uuid().v4();
      final scope = BackupScopeIndicator(
        scope: BackupScope.user,
        id: username,
        backupId: backupId,
      );
      final response = await dio.post(
        backupEndpoint,
        data: {"scope": scope.toJson()},
      );
      final genericResponse = GenericResponse.fromJson(
        response.data as Map<String, dynamic>,
      );

      return genericResponse.taskId;
    } catch (e) {
      rethrow;
    }
  }

  /// Returns task_id
  Future<String> restoreBackup(Dio dio, String backupId) async {
    try {
      final response = await dio.post(
        restoreBackupEndpoint,
        data: {"backup_id": backupId},
      );
      final genericResponse = GenericResponse.fromJson(
        response.data as Map<String, dynamic>,
      );

      return genericResponse.taskId;
    } catch (e) {
      rethrow;
    }
  }
}
