import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:dio/dio.dart';
import 'package:notes/services/collection.dart';
import 'package:notes/services/document.dart';
import 'package:notes/services/backup.dart';
import 'package:notes/state/documents.dart';
import 'package:notes/state/services.dart';
import 'package:notes/state/tabs.dart';
import 'package:notes/state/tasks.dart';
import 'package:notes/state/users.dart';

class SearchHighlight {
  final String text;
  final String? chunkId;

  SearchHighlight(this.text, {this.chunkId});
}

class AppState extends ChangeNotifier
    with Services, Users, Tabs, Tasks, Documents {
  String? currentCollectionId;

  final Map<String, CollectionMetadata> collectionById = {};

  List<CollectionMetadata> get collectionsList =>
      collectionById.values.toList();
  List<BackupListItem> backups = [];

  // Search Highlights
  final Map<String, SearchHighlight> searchHighlights = {};

  void loadLastOpenedTabs() async {
    if (username == null) return;

    final (savedOpenObjectIds, activeObject) = await loadTabs(username!);

    if (savedOpenObjectIds != null && savedOpenObjectIds.isNotEmpty) {
      final metadatas = await documents.getDocumentsMetadata(
        dio,
        null,
        savedOpenObjectIds,
      );
      documentById.addEntries(metadatas.map((e) => MapEntry(e.id, e)));

      for (final id in savedOpenObjectIds) {
        openDocument(id);
      }
    }

    if (activeObject != null) {
      setActiveObject(activeObject.type, activeObject.id);
    }
  }

  Future<bool> login(String username, String password) async {
    try {
      final success = await users.login(dio, username, password);
      if (success) {
        this.username = username;
        notifyListeners();
        loadLastOpenedTabs();
        await refreshAll();
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

  Future<void> pollTasks() async {
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
                  swapDocumentId(tempId, newId);
                }
              }

              changed = true;
              await refreshAll();
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

    hasPending = tasks.any(
      (t) => t.status == 'Pending' || t.status == 'InProgress',
    );
    if (!hasPending) {
      pollingTimer?.cancel();
      pollingTimer = null;
    }
  }

  Future<void> renameCollection(String collectionId, String newTitle) async {
    final collection = collectionById[collectionId];
    if (collection == null) return;

    collection.title = newTitle;

    final taskId = await collections.updateCollectionsMetadata(dio, [
      collection,
    ]);
    addTask(taskId, "Renaming collection to '$newTitle'", pollTasks);
    notifyListeners();
  }

  Future<void> reindexDocuments() async {
    if (username == null) return;
    try {
      final taskId = await documents.reindex(dio, username!);
      addTask(taskId, "Reindexing documents", pollTasks);
      notifyListeners();
    } catch (e) {
      rethrow;
    }
  }

  @override
  void dispose() {
    pollingTimer?.cancel();
    super.dispose();
  }

  /// To refresh all data for syncing with the backend
  Future<void> refreshAll() async {
    await Future.wait([
      refreshDocuments(),
      refreshCollections(), // Also refresh collections as some tasks might affect them
      fetchBackups(),
      if (username != null)
        keyBindings.fetchAndApplyConfigurations(dio, users, username!),
    ]);
  }

  Future<void> refreshCollections() async {
    if (username == null) return;
    final list = await collections.getCollections(dio, username!);
    collectionById
      ..clear()
      ..addEntries(list.map((e) => MapEntry(e.id, e)));
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

  Future<void> createDocumentInCollection(
    String collectionId,
    String title,
  ) async {
    if (username == null) return;
    final content = title;
    final taskId = await documents.addDocument(
      dio,
      username!,
      title,
      collectionId,
      content,
    );
    addTask(taskId, "Creating document '$title'", pollTasks);
  }

  Future<void> deleteCollection(String id) async {
    await collections.deleteCollection(dio, id);
    collectionById.remove(id);
    if (currentCollectionId == id) {
      currentCollectionId = null;
    }
    await refreshCollections();
    notifyListeners();
  }

  Future<void> importDocuments(
    List<Map<String, dynamic>> imports, {
    String? collectionId,
  }) async {
    final targetCollectionId = collectionId ?? currentCollectionId;
    if (targetCollectionId == null || username == null) return;
    final taskId = await documents.importDocuments(
      dio,
      username!,
      targetCollectionId,
      imports,
    );
    addTask(taskId, "Importing ${imports.length} documents", pollTasks);
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
      addTask(taskId, "Creating backup", pollTasks);
    } catch (e) {
      rethrow;
    }
  }

  Future<void> restoreBackup(String backupId) async {
    try {
      final taskId = await backupService.restoreBackup(dio, backupId);
      addTask(taskId, "Restoring backup", pollTasks);
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

  void swapDocumentId(String oldId, String newId) {
    if (!documentById.containsKey(oldId)) return;

    final doc = documentById[oldId]!;
    final content = documentContentCache[oldId];
    final highlights = searchHighlights[oldId];

    // Update metadata ID
    doc.id = newId;

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
    final tabIndex = openObjectIds.indexOf(oldId);
    if (tabIndex != -1) {
      openObjectIds[tabIndex] = newId;
    }

    // Update Active Item
    if (activeObject.id == oldId) {
      setActiveObject(ActiveObjectType.document, newId);
    }
  }

  Future<void> openDocument(
    String documentId, {
    String? highlightText,
    String? highlightChunkId,
    String? collectionId,
  }) async {
    if (!openObjectIds.contains(documentId)) {
      openObjectIds.add(documentId);
    }

    if (highlightText != null) {
      searchHighlights[documentId] = SearchHighlight(
        highlightText,
        chunkId: highlightChunkId,
      );
    }

    // Ensure we have metadata if possible
    if (!documentById.containsKey(documentId) && collectionId != null) {
      try {
        await fetchDocumentsForCollection(collectionId);
      } catch (e) {
        print("Error fetching metadata for document opening: $e");
      }
    }

    setActiveObject(ActiveObjectType.document, documentId);
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
    final removedIndex = openObjectIds.indexOf(documentId);
    if (removedIndex == -1) return;

    openObjectIds.removeAt(removedIndex);
    searchHighlights.remove(documentId);

    final wasActiveDocument =
        activeObject.type == ActiveObjectType.document &&
        activeObject.id == documentId;
    final wasLastActive = lastActiveObjectId == documentId;

    if (openObjectIds.isEmpty) {
      lastActiveObjectId = null;
      setActiveObject(ActiveObjectType.none, null);
      return;
    }

    if (wasActiveDocument || wasLastActive) {
      final nextIndex = removedIndex < openObjectIds.length
          ? removedIndex
          : openObjectIds.length - 1;
      setActiveObject(ActiveObjectType.document, openObjectIds[nextIndex]);
      return;
    }

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
      documentsByCollectionId[oldCollectionId]?.removeWhere(
        (d) => d.id == documentId,
      );
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
    addTask(taskId, "Moving document to new collection", pollTasks);
    notifyListeners();
  }

  Future<void> refreshDocuments() async {
    if (currentCollectionId == null) return;
    final docs = await documents.getDocumentsMetadata(
      dio,
      currentCollectionId,
      null,
    );
    documentById
      ..clear()
      ..addEntries(docs.map((e) => MapEntry(e.id, e)));
    notifyListeners();
  }

  Future<void> deleteDocument(String id) async {
    final title = documentById[id]?.title ?? "document";
    final taskId = await documents.deleteDocument(dio, id);
    documentById.remove(id);

    // Remove from tree cache
    documentsByCollectionId.forEach((key, list) {
      list.removeWhere((doc) => doc.id == id);
    });

    // Remove from tabs
    closeDocument(id);

    addTask(taskId, "Deleting document '$title'", pollTasks);
  }

  void createLocalDocument(String collectionId) {
    final tempId = 'temp_doc_${DateTime.now().millisecondsSinceEpoch}';
    final now = DateTime.now().toIso8601String();
    final newDoc = DocumentMetadata(
      id: tempId,
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
    if (activeObject.type != ActiveObjectType.document ||
        activeObject.id == null ||
        username == null)
      return;

    final docId = activeObject.id!;
    final meta = documentById[docId];
    final content = documentContentCache[docId];

    if (meta == null || content == null) return;

    try {
      if (docId.startsWith('temp_doc_')) {
        // Create new document
        String title = 'Untitled';
        if (content.trim().isNotEmpty) {
          final firstLine = content.split('\n').first.trim();
          title = firstLine.substring(
            0,
            firstLine.length > 50 ? 50 : firstLine.length,
          );
          if (title.isEmpty) title = 'Untitled';
        }

        final taskId = await documents.addDocument(
          dio,
          username!,
          title,
          meta.collectionMetadataId,
          content,
        );
        taskIdToTempDocId[taskId] = docId;
        addTask(taskId, "Creating document '$title'", pollTasks);

        // Update local title immediately
        meta.title = title;
        notifyListeners();
      } else {
        final title = meta.title;
        final taskId = await documents.updateDocumentContent(
          dio,
          username!,
          docId,
          meta.collectionMetadataId,
          title,
          content,
        );

        addTask(taskId, "Updating document '$title'", pollTasks);
        notifyListeners();
      }
    } catch (e) {
      rethrow;
    }
  }

  // --- Tree View & Tab Management Methods ---

  Future<void> fetchDocumentsForCollection(String collectionId) async {
    final List<DocumentMetadata> list = await documents.getDocumentsMetadata(
      dio,
      collectionId,
      null,
    );

    // Merge with existing temp docs for this collection
    final List<DocumentMetadata> existingList =
        documentsByCollectionId[collectionId] ?? [];
    final List<DocumentMetadata> tempDocs = existingList
        .where((d) => d.isLocalDocument())
        .toList();
    final List<DocumentMetadata> combinedList = [...list, ...tempDocs];

    documentsByCollectionId[collectionId] = combinedList;
    documentById.addEntries(list.map((e) => MapEntry(e.id, e)));
    notifyListeners();
  }
}
