import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:json_annotation/json_annotation.dart';
import 'package:notes/state/services.dart';
import 'package:notes/state/users.dart';

part 'tabs.g.dart';

enum ActiveObjectType { collection, document, none }

@JsonSerializable()
class SavedTabsStates {
  ActiveObject activeObject;
  List<String> openObjectIds;
  String? lastActiveObjectId;

  SavedTabsStates({
    required this.activeObject,
    required this.openObjectIds,
    required this.lastActiveObjectId,
  });

  factory SavedTabsStates.fromJson(Map<String, dynamic> json) =>
      _$SavedTabsStatesFromJson(json);

  Map<String, dynamic> toJson() => _$SavedTabsStatesToJson(this);

  static SavedTabsStates fromString(String string) {
    final json = jsonDecode(string);
    return _$SavedTabsStatesFromJson(json);
  }

  @override
  String toString() {
    final json = toJson();
    return jsonEncode(json);
  }
}

@JsonSerializable()
class ActiveObject {
  final ActiveObjectType type;
  final String? id;

  ActiveObject(this.type, this.id);

  factory ActiveObject.fromJson(Map<String, dynamic> json) =>
      _$ActiveObjectFromJson(json);

  Map<String, dynamic> toJson() => _$ActiveObjectToJson(this);
}

mixin Tabs on ChangeNotifier, Services, Users {
  // Tab Management
  final List<String> openObjectIds = [];
  String? lastActiveObjectId;

  // Active Item Management
  ActiveObject activeObject = ActiveObject(ActiveObjectType.none, null);

  void switchDocumentTab(int offset) {
    if (openObjectIds.isEmpty) return;

    final String currentId;
    if (activeObject.type == ActiveObjectType.document &&
        activeObject.id != null &&
        openObjectIds.contains(activeObject.id)) {
      currentId = activeObject.id!;
    } else if (lastActiveObjectId != null &&
        openObjectIds.contains(lastActiveObjectId)) {
      currentId = lastActiveObjectId!;
    } else {
      currentId = openObjectIds.first;
    }

    final currentIndex = openObjectIds.indexOf(currentId) + offset;
    var safeCurrentIndex = currentIndex;

    if (currentIndex < 0) {
      safeCurrentIndex = openObjectIds.length - 1;
    } else if (currentIndex > openObjectIds.length - 1) {
      safeCurrentIndex = 0;
    }

    setActiveObject(ActiveObjectType.document, openObjectIds[safeCurrentIndex]);
  }

  void setActiveObject(ActiveObjectType type, String? id) {
    activeObject = ActiveObject(type, id);

    if (type == ActiveObjectType.document && id != null) {
      lastActiveObjectId = id;
      // We don't want to save collection nor null
      saveTabs(activeObject);
    }

    if (type == ActiveObjectType.none && id == null) {
      clearTabs();
    }

    notifyListeners();
  }

  /// This will remove all tabs under the local storage
  void clearTabs() {
    if (username != null) {
      localStorage.remove(username!);
    }
  }

  /// Persist the tab states to local storage for resuming them when user re-opened the app
  void saveTabs(ActiveObject activeObject) {
    if (activeObject.id != null &&
        lastActiveObjectId != null &&
        username != null) {
      localStorage.setString(
        username!,
        SavedTabsStates(
          activeObject: activeObject,
          openObjectIds: openObjectIds,
          lastActiveObjectId: lastActiveObjectId,
        ).toString(),
      );
    }
  }

  Future<(List<String>?, ActiveObject?)> loadTabs(String username) async {
    final String? localStorageContent = await localStorage.getString(username);

    if (localStorageContent != null) {
      final savedTabsStates = SavedTabsStates.fromString(localStorageContent);
      lastActiveObjectId = savedTabsStates.lastActiveObjectId;

      return (savedTabsStates.openObjectIds, activeObject);
    }

    return (null, null);
  }
}
