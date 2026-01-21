import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:notes/state/services.dart';

class TaskInfo {
  final String id;
  final String description;
  String status;
  String? message;
  final DateTime createdAt;

  TaskInfo({
    required this.id,
    required this.description,
    this.status = 'Pending',
    this.message,
  }) : createdAt = DateTime.now();
}

mixin Tasks on ChangeNotifier, Services {
  // Handle interactions with the tasks scheduler
  final List<TaskInfo> tasks = [];
  Timer? pollingTimer;
  
  final Map<String, String> taskStatusById = {};
  final Map<String, String> taskIdToTempDocId = {};

  // Register a new task 
  void addTask(String taskId, String description, Function pollTasks) {
    tasks.insert(0, TaskInfo(id: taskId, description: description));
    notifyListeners();
    // Immediate poll to catch fast tasks
    pollTasks();
    startPolling(pollTasks);
  }

  void startPolling(Function pollTasks) {
    if (pollingTimer != null && pollingTimer!.isActive) return;
    pollingTimer = Timer.periodic(const Duration(milliseconds: 500), (
      timer,
    ) async {
      await pollTasks();
    });
  }  
}