import 'package:flutter/foundation.dart';
import 'package:notes/services/document.dart';
import 'package:notes/state/services.dart';
import 'package:notes/state/tasks.dart';

mixin Documents on ChangeNotifier, Services, Tasks {
  final Map<String, DocumentMetadata> documentById = {};
  
  // Tree View Caches
  final Map<String, List<DocumentMetadata>> documentsByCollectionId = {};
  
  // Document Content Cache
  final Map<String, String> documentContentCache = {};
  final Map<String, Map<String, int>> documentChunkOffsets = {};
  
  // Get documents as a list
  List<DocumentMetadata> get documentsList => documentById.values.toList();
  
  void updateDocumentDraft(String docId, String content) {
    documentContentCache[docId] = content;
    documentChunkOffsets.remove(docId);
  } 
  
  Future<void> renameDocument(String documentId, String newTitle, Function pollTasks) async {
    final document = documentById[documentId];
    if (document == null) return;

    document.title = newTitle;

    final taskId = await documents.updateDocumentsMetadata(dio, [document]);
    addTask(taskId, "Renaming document to '$newTitle'", pollTasks);
    notifyListeners();
  }
}