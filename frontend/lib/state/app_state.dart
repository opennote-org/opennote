import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:dio/dio.dart';
import 'package:notes/services/collection.dart';
import 'package:notes/services/document.dart';
import 'package:notes/services/general.dart';
import 'package:notes/services/user.dart';
import 'package:notes/services/backup.dart';

class TaskInfo {
  final String id;
  final String description;
  String status;
  String? message;
  final DateTime createdAt;

  TaskInfo({required this.id, required this.description, this.status = 'Pending', this.message}) : createdAt = DateTime.now();
}

enum ActiveItemType { collection, document, none }

class ActiveItem {
  final ActiveItemType type;
  final String? id;
  ActiveItem(this.type, this.id);
}

class SearchHighlight {
  final String text;
  final String? chunkId;

  SearchHighlight(this.text, {this.chunkId});
}

class AppState extends ChangeNotifier {
  final Dio dio = Dio();
  final CollectionManagementService collections = CollectionManagementService();
  final DocumentManagementService documents = DocumentManagementService();
  final GeneralService general = GeneralService();
  final UserManagementService users = UserManagementService();
  final BackupService backupService = BackupService();

  String? username;
  String? currentCollectionId;
  String? currentDocumentId;

  final Map<String, CollectionMetadata> collectionById = {};
  final Map<String, DocumentMetadata> documentById = {};
  final Map<String, String> taskStatusById = {};
  final Map<String, String> taskIdToTempDocId = {};

  // Handle interactions with the tasks scheduler
  final List<TaskInfo> tasks = [];
  Timer? _pollingTimer;

  List<CollectionMetadata> get collectionsList => collectionById.values.toList();
  List<DocumentMetadata> get documentsList => documentById.values.toList();
  List<BackupListItem> backups = [];
  
  // Tree View Caches
  final Map<String, List<DocumentMetadata>> documentsByCollectionId = {};

  // Document Content Cache
  final Map<String, String> documentContentCache = {};
  final Map<String, Map<String, int>> documentChunkOffsets = {};

  // Search Highlights
  final Map<String, SearchHighlight> searchHighlights = {};

  // Tab Management
  final List<String> openDocumentIds = [];

  // Active Item Management
  ActiveItem _activeItem = ActiveItem(ActiveItemType.none, null);
  ActiveItem get activeItem => _activeItem;

  void setActiveItem(ActiveItemType type, String? id) {
    _activeItem = ActiveItem(type, id);
    notifyListeners();
  }

  void updateDocumentDraft(String docId, String content) {
    documentContentCache[docId] = content;
    documentChunkOffsets.remove(docId);
  }

  void createLocalDocument(String collectionId) {
    final tempId = 'temp_doc_${DateTime.now().millisecondsSinceEpoch}';
    final now = DateTime.now().toIso8601String();
    final newDoc = DocumentMetadata(
      metadataId: tempId,
      createdAt: now,
      lastModified: now,
      collectionMetadataId: collectionId,
      title: 'Untitled',
      chunks: [],
    );

    documentById[tempId] = newDoc;
    // Add to tree view cache as well so it appears in sidebar
    if (documentsByCollectionId.containsKey(collectionId)) {
      documentsByCollectionId[collectionId]!.add(newDoc);
    } else {
      documentsByCollectionId[collectionId] = [newDoc];
    }

    // Initialize empty content
    documentContentCache[tempId] = '';
    
    openDocument(tempId, collectionId: collectionId);
  }

  Future<void> saveActiveDocument() async {
    if (_activeItem.type != ActiveItemType.document || _activeItem.id == null || username == null) return;

    final docId = _activeItem.id!;
    final meta = documentById[docId];
    final content = documentContentCache[docId];

    if (meta == null || content == null) return;

    try {
      if (docId.startsWith('temp_doc_')) {
        // Create new document
        String title = 'Untitled';
        if (content.trim().isNotEmpty) {
          final firstLine = content.split('\n').first.trim();
          title = firstLine.substring(0, firstLine.length > 50 ? 50 : firstLine.length);
          if (title.isEmpty) title = 'Untitled';
        }

        final taskId = await documents.addDocument(dio, username!, title, meta.collectionMetadataId, content);
        taskIdToTempDocId[taskId] = docId;
        _addTask(taskId, "Creating document '$title'");

        // Update local title immediately
        meta.title = title;
        notifyListeners();
      } else {
        final title = meta.title;
        final taskId = await documents.updateDocumentContent(dio, username!, docId, meta.collectionMetadataId, title, content);

        _addTask(taskId, "Updating document '$title'");
        notifyListeners();
      }
    } catch (e) {
      rethrow;
    }
  }

  Future<void> renameCollection(String collectionId, String newTitle) async {
    final collection = collectionById[collectionId];
    if (collection == null) return;

    collection.title = newTitle;

    final taskId = await collections.updateCollectionsMetadata(dio, [collection]);
    _addTask(taskId, "Renaming collection to '$newTitle'");
    notifyListeners();
  }

  Future<void> renameDocument(String documentId, String newTitle) async {
    final document = documentById[documentId];
    if (document == null) return;

    document.title = newTitle;

    final taskId = await documents.updateDocumentsMetadata(dio, [document]);
    _addTask(taskId, "Renaming document to '$newTitle'");
    notifyListeners();
  }

  Future<void> moveDocument(String documentId, String newCollectionId) async {
    final document = documentById[documentId];
    if (document == null) return;

    // Optimistic update
    final oldCollectionId = document.collectionMetadataId;
    document.collectionMetadataId = newCollectionId;

    // Update tree cache
    if (documentsByCollectionId.containsKey(oldCollectionId)) {
      documentsByCollectionId[oldCollectionId]?.removeWhere((d) => d.metadataId == documentId);
    }
    if (documentsByCollectionId.containsKey(newCollectionId)) {
      documentsByCollectionId[newCollectionId]?.add(document);
    } else {
      // If the target collection is not loaded, we might want to load it or just leave it
      // The optimistic update above (document.collectionMetadataId) handles the main state
      // But the tree view relies on documentsByCollectionId
      await fetchDocumentsForCollection(newCollectionId);
    }

    final taskId = await documents.updateDocumentsMetadata(dio, [document]);
    _addTask(taskId, "Moving document to new collection");
    notifyListeners();
  }

  Future<void> reindexDocuments() async {
    if (username == null) return;
    try {
      final taskId = await documents.reindex(dio, username!);
      _addTask(taskId, "Reindexing documents");
      notifyListeners();
    } catch (e) {
      rethrow;
    }
  }

  String get appBarTitle {
    final colId = currentCollectionId;
    final docId = currentDocumentId;
    final col = colId == null ? null : (collectionById[colId]?.title ?? colId);
    final doc = docId == null ? null : (documentById[docId]?.title ?? docId);
    if (col == null) return 'Notes';
    if (doc == null) return '$col';
    return '$col > $doc';
  }

  @override
  void dispose() {
    _pollingTimer?.cancel();
    super.dispose();
  }

  void _swapDocumentId(String oldId, String newId) {
    if (!documentById.containsKey(oldId)) return;

    final doc = documentById[oldId]!;
    final content = documentContentCache[oldId];
    final highlights = searchHighlights[oldId];

    // Update metadata ID
    doc.metadataId = newId;

    // Update Maps
    documentById.remove(oldId);
    documentById[newId] = doc;

    if (content != null) {
      documentContentCache.remove(oldId);
      documentContentCache[newId] = content;
    }

    if (highlights != null) {
      searchHighlights.remove(oldId);
      searchHighlights[newId] = highlights;
    }

    // Update Open Tabs
    final tabIndex = openDocumentIds.indexOf(oldId);
    if (tabIndex != -1) {
      openDocumentIds[tabIndex] = newId;
    }

    // Update Active Item
    if (activeItem.id == oldId) {
      setActiveItem(ActiveItemType.document, newId);
    }
  }

  void _addTask(String taskId, String description) {
    tasks.insert(0, TaskInfo(id: taskId, description: description));
    notifyListeners();
    // Immediate poll to catch fast tasks
    _pollTasks();
    _startPolling();
  }

  void _startPolling() {
    if (_pollingTimer != null && _pollingTimer!.isActive) return;
    _pollingTimer = Timer.periodic(const Duration(milliseconds: 500), (timer) async {
      await _pollTasks();
    });
  }

  Future<void> _pollTasks() async {
    bool hasPending = false;
    bool changed = false;

    for (final task in tasks) {
      if (task.status == 'Pending' || task.status == 'InProgress') {
        hasPending = true;
        try {
          final result = await general.retrieveTaskResult(dio, task.id);
          final status = result.status;

          if (status != task.status) {
            if (status == 'Completed') {
              task.status = 'Success';
              task.message = result.message;

              // Handle ID swap if it was a creation task
              if (taskIdToTempDocId.containsKey(task.id)) {
                final tempId = taskIdToTempDocId[task.id]!;
                taskIdToTempDocId.remove(task.id);

                // Try to find new ID in result data
                if (result.data != null &&
                    result.data is Map &&
                    result.data['document_metadata_id'] != null) {
                  final newId = result.data['document_metadata_id'] as String;
                  _swapDocumentId(tempId, newId);
                }
              }

              changed = true;
              await refreshDocuments();
              await refreshCollections(); // Also refresh collections as some tasks might affect them
              await fetchBackups();
              // Refresh all cached collections documents to update sidebar
              for (final collectionId in documentsByCollectionId.keys) {
                await fetchDocumentsForCollection(collectionId);
              }
            } else if (status == 'Failed') {
              task.status = 'Failure';
              task.message = result.message ?? 'Unknown error';
              changed = true;
            } else if (status == 'InProgress') {
              if (task.status == 'Pending') {
                task.status = 'InProgress';
                changed = true;
              }
            }
          }
        } catch (e) {
          if (e is DioException) {
            if (e.response?.statusCode == 404) {
              task.status = 'Failure';
              task.message = 'Task not found';
              if (e.response?.data is Map<String, dynamic>) {
                final data = e.response!.data as Map<String, dynamic>;
                if (data.containsKey('message')) {
                  task.message = data['message'] as String;
                }
              }
              changed = true;
            }
          }
        }
      }
    }

    if (changed) notifyListeners();

    hasPending = tasks.any((t) => t.status == 'Pending' || t.status == 'InProgress');
    if (!hasPending) {
      _pollingTimer?.cancel();
      _pollingTimer = null;
    }
  }

  Future<bool> login(String username, String password) async {
    try {
      final success = await users.login(dio, username, password);
      if (success) {
        this.username = username;
        notifyListeners();
        await refreshCollections();
        return true;
      }
      return false;
    } catch (e) {
      return false;
    }
  }

  Future<void> register(String username, String password) async {
    await users.createUser(dio, username, password);
  }

  void logout() {
    username = null;
    currentCollectionId = null;
    currentDocumentId = null;
    collectionById.clear();
    documentById.clear();
    tasks.clear();
    _pollingTimer?.cancel();
    notifyListeners();
  }

  Future<void> refreshCollections() async {
    if (username == null) return;
    final list = await collections.getCollections(dio, username!);
    collectionById
      ..clear()
      ..addEntries(list.map((e) => MapEntry(e.metadataId, e)));
    notifyListeners();
  }

  Future<void> selectCollection(String id) async {
    currentCollectionId = id;
    notifyListeners();
  }

  Future<void> createCollection(String title) async {
    if (username == null) return;
    await collections.createCollection(dio, title, username!);
    await refreshCollections();
    notifyListeners();
  }

  Future<void> createDocumentInCollection(String collectionId, String title) async {
    if (username == null) return;
    final content = title;
    final taskId = await documents.addDocument(dio, username!, title, collectionId, content);
    _addTask(taskId, "Creating document '$title'");
  }

  Future<void> deleteCollection(String id) async {
    await collections.deleteCollection(dio, id);
    collectionById.remove(id);
    if (currentCollectionId == id) {
      currentCollectionId = null;
    }
    notifyListeners();
  }

  Future<void> refreshDocuments() async {
    if (currentCollectionId == null) return;
    final docs = await documents.getDocumentsMetadata(dio, currentCollectionId!);
    documentById
      ..clear()
      ..addEntries(docs.map((e) => MapEntry(e.metadataId, e)));
    notifyListeners();
  }

  Future<void> importDocuments(List<Map<String, dynamic>> imports, {String? collectionId}) async {
    final targetCollectionId = collectionId ?? currentCollectionId;
    if (targetCollectionId == null || username == null) return;
    final taskId = await documents.importDocuments(dio, username!, targetCollectionId, imports);
    _addTask(taskId, "Importing ${imports.length} documents");
  }

  Future<void> fetchBackups() async {
    if (username == null) return;
    try {
      backups = await backupService.getBackupsList(dio, username!);
      notifyListeners();
    } catch (e) {
      print("Failed to fetch backups: $e");
    }
  }

  Future<void> createBackup() async {
    if (username == null) return;
    try {
      final taskId = await backupService.backup(dio, username!);
      _addTask(taskId, "Creating backup");
    } catch (e) {
      rethrow;
    }
  }

  Future<void> restoreBackup(String backupId) async {
    try {
      final taskId = await backupService.restoreBackup(dio, backupId);
      _addTask(taskId, "Restoring backup");
    } catch (e) {
      rethrow;
    }
  }

  Future<void> deleteBackup(String backupId) async {
    try {
      await backupService.removeBackups(dio, [backupId]);
      await fetchBackups();
    } catch (e) {
      rethrow;
    }
  }

  Future<void> deleteDocument(String id) async {
    final title = documentById[id]?.title ?? "document";
    final taskId = await documents.deleteDocument(dio, id);
    documentById.remove(id);
    if (currentDocumentId == id) {
      currentDocumentId = null;
    }

    // Remove from tree cache
    documentsByCollectionId.forEach((key, list) {
      list.removeWhere((doc) => doc.metadataId == id);
    });

    // Remove from tabs
    closeDocument(id);

    _addTask(taskId, "Deleting document '$title'");
  }

  // --- Tree View & Tab Management Methods ---

  Future<void> fetchDocumentsForCollection(String collectionId) async {
    final list = await documents.getDocumentsMetadata(dio, collectionId);
    
    // Merge with existing temp docs for this collection
    final existingList = documentsByCollectionId[collectionId] ?? [];
    final tempDocs = existingList.where((d) => d.metadataId.startsWith('temp_doc_')).toList();
    final combinedList = [...list, ...tempDocs];

    documentsByCollectionId[collectionId] = combinedList;
    documentById.addEntries(list.map((e) => MapEntry(e.metadataId, e)));
    notifyListeners();
  }

  Future<void> openDocument(String documentId, {String? highlightText, String? highlightChunkId, String? collectionId}) async {
    if (!openDocumentIds.contains(documentId)) {
      openDocumentIds.add(documentId);
    }

    if (highlightText != null) {
      searchHighlights[documentId] = SearchHighlight(highlightText, chunkId: highlightChunkId);
    }

    // Ensure we have metadata if possible
    if (!documentById.containsKey(documentId) && collectionId != null) {
      try {
        await fetchDocumentsForCollection(collectionId);
      } catch (e) {
        print("Error fetching metadata for document opening: $e");
      }
    }

    setActiveItem(ActiveItemType.document, documentId);
    notifyListeners();

    if (!documentContentCache.containsKey(documentId)) {
      try {
        final chunks = await documents.getDocument(dio, documentId);
        final fullContent = chunks.map((c) => c.content).join('');
        documentContentCache[documentId] = fullContent;

        // Calculate chunk offsets
        int currentOffset = 0;
        final Map<String, int> offsets = {};
        for (final chunk in chunks) {
          offsets[chunk.id] = currentOffset;
          currentOffset += chunk.content.length;
        }
        documentChunkOffsets[documentId] = offsets;

        notifyListeners();
      } catch (e) {
        print("Error loading document content: $e");
      }
    }
  }

  void closeDocument(String documentId) {
    openDocumentIds.remove(documentId);
    searchHighlights.remove(documentId);
    if (activeItem.id == documentId) {
      if (activeItem.id != null) {
        setActiveItem(ActiveItemType.document, activeItem.id);
      } else {
        setActiveItem(ActiveItemType.none, null);
      }
    }
    notifyListeners();
  }
}
