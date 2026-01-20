import 'package:dio/dio.dart';
import 'package:notes/services/collection.dart';
import 'package:notes/services/document.dart';
import 'package:notes/services/general.dart';
import 'package:notes/services/user.dart';
import 'package:notes/services/backup.dart';
import 'package:notes/services/key_mapping.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// All services will be registered automatically 
mixin Services {
  final Dio dio = Dio();
  final CollectionManagementService collections = CollectionManagementService();
  final DocumentManagementService documents = DocumentManagementService();
  final GeneralService general = GeneralService();
  final UserManagementService users = UserManagementService();
  final BackupService backupService = BackupService();
  final KeyBindingService keyBindings = KeyBindingService();
  final SharedPreferencesAsync localStorage = SharedPreferencesAsync();
}
