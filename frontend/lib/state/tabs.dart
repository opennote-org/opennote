import 'package:flutter/foundation.dart';
import 'package:notes/state/services.dart';

// Keys of the states in local storage
const String activeObjectTypeKey = "activeObjectType";
const String activeObjectIdKey = "id";
const String openObjectIdsKey = "openObjectIds";
const String lastActiveObjectIdKey = "lastActiveObjectId";

enum ActiveObjectType { collection, document, none }

class ActiveObject {
  final ActiveObjectType type;
  final String? id;
  ActiveObject(this.type, this.id);
}

mixin Tabs on ChangeNotifier, Services {
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
    }

    saveTabs();
    notifyListeners();
  }

  /// Persist the tab states to local storage for resuming them when user re-opened the app
  void saveTabs() {
    if (activeObject.id != null) {
      localStorage.setString(activeObjectTypeKey, activeObject.type.name);
      localStorage.setString(activeObjectIdKey, activeObject.id!);
    }

    localStorage.setStringList(openObjectIdsKey, openObjectIds);
    if (lastActiveObjectId != null) {
      localStorage.setString(lastActiveObjectIdKey, lastActiveObjectId!);
    }
  }

  Future<(List<String>?, ActiveObject?)> loadTabs() async {
    final activeObjectTypeString = await localStorage.getString(
      activeObjectTypeKey,
    );
    final activeObjectId = await localStorage.getString(activeObjectIdKey);
    final savedOpenObjectIds = await localStorage.getStringList(
      openObjectIdsKey,
    );
    final savedLastActiveObjectId = await localStorage.getString(
      lastActiveObjectIdKey,
    );

    if (savedOpenObjectIds != null) {
      if (activeObjectTypeString != null && activeObjectId != null) {
        final type = ActiveObjectType.values.firstWhere(
          (e) => e.name == activeObjectTypeString,
          orElse: () => ActiveObjectType.none,
        );
        activeObject = ActiveObject(type, activeObjectId);
      } else {
        activeObject = ActiveObject(ActiveObjectType.none, null);
      }

      lastActiveObjectId = savedLastActiveObjectId;

      return (savedOpenObjectIds, activeObject);
    }

    return (null, null);
  }
}
