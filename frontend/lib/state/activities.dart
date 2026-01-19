enum ActiveItemType { collection, document, none }

class ActiveItem {
  final ActiveItemType type;
  final String? id;
  ActiveItem(this.type, this.id);
}

class Activities {
  // Tab Management
  final List<String> openDocumentIds = [];
  String? lastActiveDocumentId;

  // Active Item Management
  ActiveItem _activeItem = ActiveItem(ActiveItemType.none, null);
  ActiveItem get activeItem => _activeItem;

  void switchDocumentTab(int offset) {
    if (openDocumentIds.isEmpty) return;

    final String currentId;
    if (activeItem.type == ActiveItemType.document && activeItem.id != null && openDocumentIds.contains(activeItem.id)) {
      currentId = activeItem.id!;
    } else if (lastActiveDocumentId != null && openDocumentIds.contains(lastActiveDocumentId)) {
      currentId = lastActiveDocumentId!;
    } else {
      currentId = openDocumentIds.first;
    }

    final currentIndex = openDocumentIds.indexOf(currentId) + offset;
    var safeCurrentIndex = currentIndex;

    if (currentIndex < 0) {
      safeCurrentIndex = openDocumentIds.length - 1;
    } else if (currentIndex > openDocumentIds.length - 1) {
      safeCurrentIndex = 0;
    }

    setActiveItem(ActiveItemType.document, openDocumentIds[safeCurrentIndex]);
  }
  
  void setActiveItem(ActiveItemType type, String? id, Function changeNotifier) {
    _activeItem = ActiveItem(type, id);
    if (type == ActiveItemType.document && id != null) {
      lastActiveDocumentId = id;
    }
    changeNotifier();
  }
}
