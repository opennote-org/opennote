import 'dart:convert';
import 'package:dio/dio.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:notes/screens/search/search_popup.dart';
import 'package:notes/services/search.dart';
import 'package:notes/state/app_state.dart';
import 'package:notes/state/app_state_scope.dart';
import 'package:notes/widgets/content_area.dart';
import 'package:notes/widgets/notification_center.dart';
import 'package:notes/widgets/sidebar.dart';

class MainScreen extends StatefulWidget {
  const MainScreen({super.key});

  @override
  State<MainScreen> createState() => _MainScreenState();
}

class _MainScreenState extends State<MainScreen> {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      AppStateScope.of(context).refreshCollections();
    });
  }

  // --- Import Logic ---
  bool _isLoading = false;

  Future<void> _performImport(List<Map<String, dynamic>> imports) async {
    if (imports.isEmpty) return;
    setState(() => _isLoading = true);
    try {
      await AppStateScope.of(context).importDocuments(imports);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
            content: Text('Import started, please come back later...'),
          ),
        );
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Failed to import documents: $e')),
        );
      }
    } finally {
      if (mounted) setState(() => _isLoading = false);
    }
  }

  Future<void> _importWebpages() async {
    final controller = TextEditingController();
    final urls = await showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Import Webpages'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Text('Enter URLs to import (one per line):'),
            const SizedBox(height: 8),
            TextField(
              controller: controller,
              decoration: const InputDecoration(
                border: OutlineInputBorder(),
                hintText: 'https://example.com\nhttps://example.org',
              ),
              maxLines: 5,
              autofocus: true,
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, controller.text),
            child: const Text('Import'),
          ),
        ],
      ),
    );

    if (urls != null && urls.isNotEmpty) {
      if (!mounted) return;
      final urlList = urls
          .split('\n')
          .where((s) => s.trim().isNotEmpty)
          .map((s) => s.trim())
          .toList();
      if (urlList.isEmpty) return;

      final imports = urlList
          .map((url) => {"import_type": "Webpage", "artifact": url})
          .toList();

      await _performImport(imports);
    }
  }

  Future<void> _importTextFile() async {
    try {
      final result = await FilePicker.platform.pickFiles(
        allowMultiple: true,
        type: FileType.custom,
        allowedExtensions: ['txt', 'md', 'json', 'xml', 'csv'],
        withData: true,
      );

      if (result != null && result.files.isNotEmpty) {
        if (!mounted) return;

        setState(() => _isLoading = true);

        const int maxBatchSize = 2 * 1024 * 1024; // 2MB
        int currentBatchSize = 0;
        List<Map<String, dynamic>> currentBatch = [];
        int successBatches = 0;
        int failedBatches = 0;

        for (final file in result.files) {
          if (file.bytes != null) {
            final content = utf8.decode(file.bytes!);
            final fileSize = file.bytes!.length;

            if (currentBatch.isNotEmpty &&
                (currentBatchSize + fileSize > maxBatchSize)) {
              try {
                await AppStateScope.of(context).importDocuments(currentBatch);
                successBatches++;
              } catch (e) {
                failedBatches++;
                if (mounted) {
                  _handleImportError(e);
                }
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
            await AppStateScope.of(context).importDocuments(currentBatch);
            successBatches++;
          } catch (e) {
            failedBatches++;
            if (mounted) {
              _handleImportError(e);
            }
          }
        }

        if (mounted) {
          if (successBatches > 0) {
            ScaffoldMessenger.of(context).showSnackBar(
              SnackBar(
                content: Text('Started $successBatches import batches.'),
              ),
            );
          }
        }
      }
    } catch (e) {
      if (mounted) {
        _handleImportError(e);
      }
    } finally {
      if (mounted) setState(() => _isLoading = false);
    }
  }

  void _handleImportError(dynamic e) {
    String message = 'Failed to import documents: $e';
    if (e is DioException) {
      if (e.response?.statusCode == 413) {
        message = 'File too large to upload. Please try smaller files.';
      } else if (e.message != null) {
        message = e.message!;
      }
    }
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(message), backgroundColor: Colors.red),
    );
  }

  Future<void> _importDatabase() async {
    final dbTypeController = TextEditingController(text: 'mysql');
    final hostController = TextEditingController(text: 'localhost');
    final portController = TextEditingController(text: '3306');
    final userController = TextEditingController();
    final passwordController = TextEditingController();
    final dbNameController = TextEditingController();
    final tableNameController = TextEditingController();
    final queryController = TextEditingController(text: 'SELECT * FROM table');
    final columnController = TextEditingController(text: 'content_column');

    final result = await showDialog<Map<String, dynamic>>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Import from Database'),
        content: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextField(
                controller: dbTypeController,
                decoration: const InputDecoration(
                  labelText: 'Database Type (mysql, postgres, sqlite)',
                ),
              ),
              Row(
                children: [
                  Expanded(
                    child: TextField(
                      controller: hostController,
                      decoration: const InputDecoration(labelText: 'Host'),
                    ),
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: TextField(
                      controller: portController,
                      decoration: const InputDecoration(labelText: 'Port'),
                    ),
                  ),
                ],
              ),
              TextField(
                controller: userController,
                decoration: const InputDecoration(labelText: 'Username'),
              ),
              TextField(
                controller: passwordController,
                decoration: const InputDecoration(labelText: 'Password'),
                obscureText: true,
              ),
              TextField(
                controller: dbNameController,
                decoration: const InputDecoration(labelText: 'Database Name'),
              ),
              TextField(
                controller: tableNameController,
                decoration: const InputDecoration(labelText: 'Table Name'),
              ),
              TextField(
                controller: queryController,
                decoration: const InputDecoration(labelText: 'Query'),
              ),
              TextField(
                controller: columnController,
                decoration: const InputDecoration(labelText: 'Content Column'),
              ),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () {
              final artifact = {
                "database_type": dbTypeController.text,
                "host": hostController.text,
                "port": portController.text,
                "username": userController.text,
                "password": passwordController.text,
                "database_name": dbNameController.text,
                "query": queryController.text,
                "column_to_fetch": columnController.text,
                "table_name": tableNameController.text.isEmpty
                    ? null
                    : tableNameController.text,
              };
              Navigator.pop(context, artifact);
            },
            child: const Text('Import'),
          ),
        ],
      ),
    );

    if (result != null) {
      if (!mounted) return;
      final imports = [
        {"import_type": "RelationshipDatabase", "artifact": result},
      ];
      await _performImport(imports);
    }
  }

  Future<void> _saveActiveDocument() async {
    final appState = AppStateScope.of(context);
    setState(
      () => _isLoading = true,
    ); // Using _isLoading for general busy state
    try {
      await appState.saveActiveDocument();
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(const SnackBar(content: Text('Document saved')));
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text('Failed to save document: $e')));
      }
    } finally {
      if (mounted) setState(() => _isLoading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final appState = AppStateScope.of(context);
    final activeItem = appState.activeItem;
    final isDocumentActive = activeItem.type == ActiveItemType.document;

    return Scaffold(
      appBar: AppBar(
        title: const Text('Notes'),
        actions: [
          const NotificationCenterButton(),
          if (isDocumentActive)
            IconButton(
              icon: const Icon(Icons.save),
              tooltip: 'Save',
              onPressed: _isLoading ? null : _saveActiveDocument,
            ),
          if (activeItem.type == ActiveItemType.collection)
            PopupMenuButton<String>(
              tooltip: 'Import Options',
              icon: const Icon(Icons.cloud_download),
              onSelected: (value) {
                if (activeItem.id != null) {
                  appState.currentCollectionId = activeItem.id;
                } else {
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(
                      content: Text('Please select a collection first.'),
                    ),
                  );
                  return;
                }

                switch (value) {
                  case 'webpage':
                    _importWebpages();
                    break;
                  case 'textfile':
                    _importTextFile();
                    break;
                  case 'database':
                    _importDatabase();
                    break;
                }
              },
              itemBuilder: (BuildContext context) => <PopupMenuEntry<String>>[
                const PopupMenuItem<String>(
                  value: 'webpage',
                  child: ListTile(
                    leading: Icon(Icons.language),
                    title: Text('Webpage'),
                  ),
                ),
                const PopupMenuItem<String>(
                  value: 'textfile',
                  child: ListTile(
                    leading: Icon(Icons.description),
                    title: Text('Text File'),
                  ),
                ),
                const PopupMenuItem<String>(
                  value: 'database',
                  child: ListTile(
                    leading: Icon(Icons.storage),
                    title: Text('Database'),
                  ),
                ),
              ],
            ),
          if (activeItem.type != ActiveItemType.none ||
              appState.username != null)
            IconButton(
              icon: const Icon(Icons.search),
              onPressed: () {
                String? scopeId = activeItem.id;
                SearchScope scope = SearchScope.userspace;

                if (activeItem.type == ActiveItemType.collection &&
                    activeItem.id != null) {
                  scope = SearchScope.collection;
                  scopeId = activeItem.id;
                } else if (activeItem.type == ActiveItemType.document &&
                    activeItem.id != null) {
                  scope = SearchScope.document;
                  // If document is not in documentById (e.g. only in tree cache), try to find it
                  if (appState.documentById.containsKey(activeItem.id)) {
                    scopeId = appState.documentById[activeItem.id]?.metadataId;
                  } else {
                    // Fallback: Use activeItem.id directly as we expect it to be the metadataId
                    scopeId = activeItem.id;
                  }
                } else {
                  // Default to Userspace search if no specific item is active or relevant
                  scope = SearchScope.userspace;
                  scopeId = appState.username;
                }

                if (scopeId != null) {
                  showDialog(
                    context: context,
                    builder: (context) =>
                        SearchPopup(scope: scope, scopeId: scopeId!),
                  );
                } else {
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(
                      content: Text('Unable to determine search scope.'),
                    ),
                  );
                }
              },
            ),
        ],
      ),
      drawer: const Drawer(child: Sidebar()),
      body: _isLoading
          ? const Center(child: CircularProgressIndicator())
          : const ContentArea(),
    );
  }
}
