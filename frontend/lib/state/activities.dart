import 'package:flutter/foundation.dart';
import 'package:json_annotation/json_annotation.dart';
import 'package:notes/json_serialization.dart';
import 'package:notes/state/services.dart';
import 'package:notes/state/users.dart';

part 'activities.g.dart';

enum ActiveObjectType { collection, document, none }

@JsonSerializable()
class SavedTabsStates with JsonSerializationMixin {
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

mixin Activities on ChangeNotifier, Services, Users {
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
      removeDataFromLocalStorage('saved_tabs', username!);
    }
  }

  /// Persist the tab states to local storage for resuming them when user re-opened the app
  void saveTabs(ActiveObject activeObject) {
    if (activeObject.id != null &&
        lastActiveObjectId != null &&
        username != null) {
      setDataToLocalStorage(
        'saved_tabs',
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
    final String? localStorageContent = await readDataFromLocalStorage(
      'saved_tabs',
      username,
    );

    if (localStorageContent != null) {
      final savedTabsStates = JsonSerializationMixin.fromString(
        localStorageContent,
        SavedTabsStates.fromJson,
      );
      lastActiveObjectId = savedTabsStates.lastActiveObjectId;

      return (savedTabsStates.openObjectIds, savedTabsStates.activeObject);
    }

    return (null, null);
  }
}
