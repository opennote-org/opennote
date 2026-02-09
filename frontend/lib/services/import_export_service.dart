import 'dart:convert';
import 'package:archive/archive.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/foundation.dart';
import 'package:notes/services/document.dart';
import 'package:notes/utils/downloader.dart';
import 'package:notes/utils/file_utils.dart';

class ImportExportService {
  static Future<void> importTextFiles({
    required Future<void> Function(List<Map<String, dynamic>>) onBatchReady,
    required Function(dynamic) onError,
    required Function(int) onSuccess,
  }) async {
    try {
      final result = await FilePicker.platform.pickFiles(
        allowMultiple: true,
        type: FileType.custom,
        allowedExtensions: ['txt', 'md', 'json', 'xml', 'csv'],
        withData: true,
      );

      if (result != null && result.files.isNotEmpty) {
        const int maxBatchSize = 2 * 1024 * 1024; // 2MB
        int currentBatchSize = 0;
        List<Map<String, dynamic>> currentBatch = [];
        int successBatches = 0;

        for (final file in result.files) {
          if (file.bytes != null) {
            final content = utf8.decode(file.bytes!);
            final fileSize = file.bytes!.length;

            if (currentBatch.isNotEmpty &&
                (currentBatchSize + fileSize > maxBatchSize)) {
              try {
                await onBatchReady(List.from(currentBatch));
                successBatches++;
              } catch (e) {
                onError(e);
              }
              currentBatch.clear();
              currentBatchSize = 0;
            }

            currentBatch.add({"import_type": "TextFile", "artifact": content});
            currentBatchSize += fileSize;
          }
        }

        if (currentBatch.isNotEmpty) {
          try {
            await onBatchReady(List.from(currentBatch));
            successBatches++;
          } catch (e) {
            onError(e);
          }
        }

        onSuccess(successBatches);
      }
    } catch (e) {
      onError(e);
    }
  }

  static Future<void> importWebpages({
    required String content,
    required bool preserveImage,
    required Future<void> Function(List<Map<String, dynamic>>) onBatchReady,
    required Function(dynamic) onError,
    required Function(int) onSuccess,
  }) async {
    try {
      final urlList = content
          .split('\n')
          .where((s) => s.trim().isNotEmpty)
          .map((s) => s.trim())
          .toList();

      if (urlList.isEmpty) {
        onSuccess(0);
        return;
      }

      final imports = urlList
          .map((url) => {
                "import_type": "Webpage",
                "artifact": {
                  "url": url,
                  "preserve_image": preserveImage,
                }
              })
          .toList();

      await onBatchReady(imports);
      onSuccess(1);
    } catch (e) {
      onError(e);
    }
  }

  static Future<void> exportCollection({
    required List<DocumentMetadata> documents,
    required String collectionTitle,
    required Future<String> Function(String docId) fetchDocumentContent,
    required Function(int success, int failed) onComplete,
    required Function(dynamic) onError,
  }) async {
    try {
      if (documents.isEmpty) {
        onComplete(0, 0);
        return;
      }

      int successCount = 0;
      int failCount = 0;
      final archive = Archive();

      final futures = documents.map((doc) async {
        try {
          final content = await fetchDocumentContent(doc.id);
          return (doc, content);
        } catch (e) {
          debugPrint('Failed to export document ${doc.title}: $e');
          return null;
        }
      });

      final results = await Future.wait(futures);

      for (final result in results) {
        if (result != null) {
          final doc = result.$1;
          final content = result.$2;

          final safeTitle = FileUtils.sanitizeFilename(doc.title);
          final fileName = '$safeTitle.md';

          final contentBytes = utf8.encode(content);
          archive.addFile(
            ArchiveFile(fileName, contentBytes.length, contentBytes),
          );
          successCount++;
        } else {
          failCount++;
        }
      }

      if (successCount > 0) {
        final zipEncoder = ZipEncoder();
        final encodedArchive = zipEncoder.encode(archive);

        if (encodedArchive != null) {
          final safeCollectionTitle = FileUtils.sanitizeFilename(collectionTitle);
          final zipFileName = '$safeCollectionTitle.zip';
          await downloadFile(encodedArchive, zipFileName);
        }
      }

      onComplete(successCount, failCount);
    } catch (e) {
      onError(e);
    }
  }
}
