import 'dart:convert';
import 'package:dio/dio.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:notes/services/collection.dart';
import 'package:notes/services/document.dart';
import 'package:notes/state/app_state.dart';
import 'package:notes/state/app_state_scope.dart';
import 'package:notes/widgets/configuration_popup.dart';

class Sidebar extends StatelessWidget {
  const Sidebar({super.key});

  @override
  Widget build(BuildContext context) {
    final appState = AppStateScope.of(context);
    final collections = appState.collectionsList;

    return Container(
      color: Theme.of(context).colorScheme.surfaceContainerLow,
      child: Column(
        children: [
          Padding(
            padding: const EdgeInsets.all(8.0),
            child: Row(
              children: [
                const Icon(Icons.description),
                const SizedBox(width: 8),
                Text('Doc Tree', style: Theme.of(context).textTheme.titleMedium),
                const Spacer(),
                IconButton(
                  icon: const Icon(Icons.add),
                  onPressed: () async {
                    final title = await _showNameDialog(context, 'New Collection');
                    if (title != null && title.isNotEmpty) {
                      appState.createCollection(title);
                    }
                  },
                ),
              ],
            ),
          ),
          Expanded(
            child: ListView.builder(
              itemCount: collections.length,
              itemBuilder: (context, index) {
                return CollectionNode(collection: collections[index]);
              },
            ),
          ),
          const Divider(height: 1),
          Padding(
            padding: const EdgeInsets.all(8.0),
            child: SizedBox(
              width: double.infinity,
              child: TextButton.icon(
                onPressed: () {
                  showDialog(context: context, builder: (context) => const ConfigurationPopup());
                },
                icon: const Icon(Icons.settings),
                label: const Text('Configuration'),
                style: TextButton.styleFrom(alignment: Alignment.centerLeft),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

Future<String?> _showNameDialog(BuildContext context, String title, {String? initialValue}) {
  final controller = TextEditingController(text: initialValue);
  return showDialog<String>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text(title),
      content: TextField(
        controller: controller,
        autofocus: true,
        decoration: const InputDecoration(labelText: 'Name'),
      ),
      actions: [
        TextButton(onPressed: () => Navigator.pop(context), child: const Text('Cancel')),
        FilledButton(onPressed: () => Navigator.pop(context, controller.text), child: const Text('Confirm')),
      ],
    ),
  );
}

class CollectionNode extends StatefulWidget {
  final CollectionMetadata collection;
  const CollectionNode({super.key, required this.collection});

  @override
  State<CollectionNode> createState() => _CollectionNodeState();
}

class _CollectionNodeState extends State<CollectionNode> {
  bool _isExpanded = false;
  bool _isLoading = false;

  Future<void> _performImport(List<Map<String, dynamic>> imports, String collectionId) async {
    if (imports.isEmpty) return;
    setState(() => _isLoading = true);
    try {
      await AppStateScope.of(context).importDocuments(imports, collectionId: collectionId);
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

  Future<void> _importWebpages(String collectionId) async {
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

      await _performImport(imports, collectionId);
    }
  }

  Future<void> _importTextFile(String collectionId) async {
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
                await AppStateScope.of(context).importDocuments(currentBatch, collectionId: collectionId);
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
            await AppStateScope.of(context).importDocuments(currentBatch, collectionId: collectionId);
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

  Future<void> _importDatabase(String collectionId) async {
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
      await _performImport(imports, collectionId);
    }
  }

  void _showImportOptionsDialog(String collectionId) {
    showDialog(
      context: context,
      builder: (context) => SimpleDialog(
        title: const Text('Import Options'),
        children: [
          SimpleDialogOption(
            onPressed: () {
              Navigator.pop(context);
              _importWebpages(collectionId);
            },
            child: const ListTile(
              leading: Icon(Icons.language),
              title: Text('Webpage'),
            ),
          ),
          SimpleDialogOption(
            onPressed: () {
              Navigator.pop(context);
              _importTextFile(collectionId);
            },
            child: const ListTile(
              leading: Icon(Icons.description),
              title: Text('Text File'),
            ),
          ),
          SimpleDialogOption(
            onPressed: () {
              Navigator.pop(context);
              _importDatabase(collectionId);
            },
            child: const ListTile(
              leading: Icon(Icons.storage),
              title: Text('Database'),
            ),
          ),
        ],
      ),
    );
  }

  void _showDocumentMenu(BuildContext context, Offset position, DocumentMetadata doc) {
    final appState = AppStateScope.of(context);
    showMenu(
      context: context,
      position: RelativeRect.fromLTRB(position.dx, position.dy, position.dx, position.dy),
      items: [
        const PopupMenuItem(value: 'rename', child: Text('Rename')),
        const PopupMenuItem(value: 'delete', child: Text('Delete')),
      ],
    ).then((value) async {
      if (value == 'delete') {
        appState.deleteDocument(doc.metadataId);
      } else if (value == 'rename') {
        final title = await _showNameDialog(context, 'Rename Document', initialValue: doc.title);
        if (title != null && title.isNotEmpty) {
          appState.renameDocument(doc.metadataId, title);
        }
      }
    });
  }

  void _showCollectionMenu(BuildContext context, Offset position) {
    final appState = AppStateScope.of(context);
    showMenu(
      context: context,
      position: RelativeRect.fromLTRB(position.dx, position.dy, position.dx, position.dy),
      items: [
        const PopupMenuItem(value: 'create_document', child: Text('New Document')),
        const PopupMenuItem(value: 'rename', child: Text('Rename Collection')),
        const PopupMenuItem(value: 'delete', child: Text('Delete Collection')),
        const PopupMenuItem(value: 'import', child: Text('Import')),
      ],
    ).then((value) async {
      if (value == 'delete') {
        appState.deleteCollection(widget.collection.metadataId);
      } else if (value == 'rename') {
        if (!mounted) return;
        final title = await _showNameDialog(context, 'Rename Collection', initialValue: widget.collection.title);
        if (title != null && title.isNotEmpty) {
          appState.renameCollection(widget.collection.metadataId, title);
        }
      } else if (value == 'create_document') {
        if (!mounted) return;
        final title = await _showNameDialog(context, 'New Document');
        if (title != null && title.isNotEmpty) {
          if (mounted) {
            appState.createDocumentInCollection(widget.collection.metadataId, title);

            setState(() {
              _isExpanded = true;
            });
            // Ensure documents are loaded
            appState.fetchDocumentsForCollection(widget.collection.metadataId);
          }
        }
      } else if (value == 'import') {
        _showImportOptionsDialog(widget.collection.metadataId);
      }
    });
  }

  void _toggleExpansion() {
    setState(() {
      _isExpanded = !_isExpanded;
    });
    if (_isExpanded) {
      final appState = AppStateScope.of(context);
      appState.setActiveItem(ActiveItemType.collection, widget.collection.metadataId);
      appState.fetchDocumentsForCollection(widget.collection.metadataId);
    }
  }

  @override
  Widget build(BuildContext context) {
    final appState = AppStateScope.of(context);
    final documents = appState.documentsByCollectionId[widget.collection.metadataId] ?? [];

    return DragTarget<DocumentMetadata>(
      onWillAccept: (data) => data != null && data.collectionMetadataId != widget.collection.metadataId,
      onAccept: (data) {
        appState.moveDocument(data.metadataId, widget.collection.metadataId);
      },
      builder: (context, candidateData, rejectedData) {
        return Column(
          children: [
            GestureDetector(
              onTap: _toggleExpansion,
              onSecondaryTapDown: (details) {
                appState.setActiveItem(ActiveItemType.collection, widget.collection.metadataId);
                _showCollectionMenu(context, details.globalPosition);
              },
              child: Container(
                color: candidateData.isNotEmpty ? Theme.of(context).colorScheme.primaryContainer : null,
                child: ListTile(
                  title: Row(
                    children: [
                      Icon(_isExpanded ? Icons.expand_more : Icons.chevron_right, size: 20, color: Theme.of(context).iconTheme.color),
                      const SizedBox(width: 8),
                      Expanded(child: Text(widget.collection.title)),
                      Builder(
                        builder: (context) {
                          return IconButton(
                            icon: const Icon(Icons.more_vert, size: 16),
                            onPressed: () {
                              final renderBox = context.findRenderObject() as RenderBox;
                              final offset = renderBox.localToGlobal(Offset.zero);
                              _showCollectionMenu(context, offset + Offset(0, renderBox.size.height));
                            },
                          );
                        },
                      ),
                    ],
                  ),
                  selected: appState.activeItem.type == ActiveItemType.collection && appState.activeItem.id == widget.collection.metadataId,
                ),
              ),
            ),
            if (_isExpanded)
              Padding(
                padding: const EdgeInsets.only(left: 12.0),
                child: Column(
                  children: documents.map((doc) {
                    return Draggable<DocumentMetadata>(
                      data: doc,
                      feedback: Material(
                        elevation: 4.0,
                        child: Container(
                          padding: const EdgeInsets.all(8.0),
                          color: Theme.of(context).colorScheme.surface,
                          child: Text(doc.title),
                        ),
                      ),
                      childWhenDragging: Opacity(opacity: 0.5, child: _buildDocumentTile(context, doc, appState)),
                      child: _buildDocumentTile(context, doc, appState),
                    );
                  }).toList(),
                ),
              ),
          ],
        );
      },
    );
  }

  Widget _buildDocumentTile(BuildContext context, DocumentMetadata doc, AppState appState) {
    return GestureDetector(
      onSecondaryTapDown: (details) {
        appState.setActiveItem(ActiveItemType.document, doc.metadataId);
        showMenu(
          context: context,
          position: RelativeRect.fromLTRB(
            details.globalPosition.dx,
            details.globalPosition.dy,
            details.globalPosition.dx,
            details.globalPosition.dy,
          ),
          items: [
            const PopupMenuItem(value: 'rename', child: Text('Rename')),
            const PopupMenuItem(value: 'delete', child: Text('Delete')),
          ],
        ).then((value) async {
          if (value == 'delete') {
            appState.deleteDocument(doc.metadataId);
          } else if (value == 'rename') {
            final title = await _showNameDialog(context, 'Rename Document', initialValue: doc.title);
            if (title != null && title.isNotEmpty) {
              appState.renameDocument(doc.metadataId, title);
            }
          }
        });
      },
      child: ListTile(
        title: Text(doc.title),
        leading: const Icon(Icons.article),
        selected: appState.activeItem.type == ActiveItemType.document && appState.activeItem.id == doc.metadataId,
        onTap: () {
          appState.openDocument(doc.metadataId);
        },
        trailing: Builder(
          builder: (context) {
            return IconButton(
              icon: const Icon(Icons.more_vert, size: 16),
              onPressed: () {
                final renderBox = context.findRenderObject() as RenderBox;
                final offset = renderBox.localToGlobal(Offset.zero);
                _showDocumentMenu(
                  context,
                  offset + Offset(0, renderBox.size.height),
                  doc,
                );
              },
            );
          },
        ),
      ),
    );
  }
}
