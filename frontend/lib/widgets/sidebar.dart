import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:notes/actions/popups.dart';
import 'package:notes/services/collection.dart';
import 'package:notes/services/document.dart';
import 'package:notes/services/import_export_service.dart';
import 'package:notes/services/key_mapping.dart';
import 'package:notes/state/activities.dart';
import 'package:notes/state/app_state.dart';
import 'package:notes/state/app_state_scope.dart';
import 'package:notes/widgets/configuration_popup.dart';
import 'package:notes/widgets/dialogs/import_database_dialog.dart';
import 'package:notes/widgets/dialogs/import_webpage_dialog.dart';

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
                Text(
                  'Doc Tree',
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const Spacer(),
                ExcludeFocus(
                  child: IconButton(
                    icon: const Icon(Icons.add),
                    onPressed: () async {
                      final title = await showNameDialog(
                        context,
                        'New Collection',
                      );
                      if (title != null && title.isNotEmpty) {
                        appState.createCollection(title);
                      }
                    },
                  ),
                ),
              ],
            ),
          ),
          Expanded(
            child: ListView.builder(
              itemCount: collections.length,
              itemBuilder: (context, index) {
                return CollectionNode(
                  collection: collections[index],
                  autofocus: index == 0,
                );
              },
            ),
          ),
          const Divider(height: 1),
          Padding(
            padding: const EdgeInsets.all(8.0),
            child: SizedBox(
              width: double.infinity,
              child: ExcludeFocus(
                child: TextButton.icon(
                  onPressed: () {
                    showDialog(
                      context: context,
                      builder: (context) => const ConfigurationPopup(),
                    );
                  },
                  icon: const Icon(Icons.settings),
                  label: const Text('Configuration'),
                  style: TextButton.styleFrom(alignment: Alignment.centerLeft),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class CollectionNode extends StatefulWidget {
  final CollectionMetadata collection;
  final bool autofocus;
  const CollectionNode({
    super.key,
    required this.collection,
    this.autofocus = false,
  });

  @override
  State<CollectionNode> createState() => _CollectionNodeState();
}

class _CollectionNodeState extends State<CollectionNode> {
  bool _isExpanded = false;
  bool _isLoading = false;
  final FocusNode _focusCollectionListTiles = FocusNode(skipTraversal: true);
  late final FocusNode _tileFocusNode;

  @override
  void initState() {
    super.initState();
    _tileFocusNode = FocusNode(onKeyEvent: _handleKeyEvent);
    _tileFocusNode.addListener(_onFocusChange);
  }

  KeyEventResult _handleKeyEvent(FocusNode node, KeyEvent event) {
    final appState = AppStateScope.of(context);

    // Only handle if Vim is enabled
    if (!appState.keyBindings.isVimEnabled) {
      return KeyEventResult.ignored;
    }

    final (action, _) = appState.keyBindings.resolve(event);

    if (action != null) {
      if (action == AppAction.cursorMoveDown) {
        FocusScope.of(context).focusInDirection(TraversalDirection.down);
        return KeyEventResult.handled;
      } else if (action == AppAction.cursorMoveUp) {
        FocusScope.of(context).focusInDirection(TraversalDirection.up);
        return KeyEventResult.handled;
      } else if (action == AppAction.cursorMoveRight) {
        if (!_isExpanded) {
          _toggleExpansion();
          return KeyEventResult.handled;
        }
      } else if (action == AppAction.cursorMoveLeft) {
        if (_isExpanded) {
          _toggleExpansion();
          return KeyEventResult.handled;
        }
      }
    }

    return KeyEventResult.ignored;
  }

  void _onFocusChange() {
    if (mounted) setState(() {});
  }

  @override
  void dispose() {
    _tileFocusNode.removeListener(_onFocusChange);
    _focusCollectionListTiles.dispose();
    _tileFocusNode.dispose();
    super.dispose();
  }

  Future<void> _performImport(
    List<Map<String, dynamic>> imports,
    String collectionId,
  ) async {
    if (imports.isEmpty) return;
    setState(() => _isLoading = true);
    try {
      await AppStateScope.of(
        context,
      ).importDocuments(imports, collectionId: collectionId);
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
    final result = await showDialog<ImportWebpageResult>(
      context: context,
      builder: (context) => const ImportWebpageDialog(),
    );

    if (result != null && result.text.isNotEmpty) {
      if (!mounted) return;
      final urlList = result.text
          .split('\n')
          .where((s) => s.trim().isNotEmpty)
          .map((s) => s.trim())
          .toList();
      if (urlList.isEmpty) return;

      final imports = urlList
          .map((url) => {
                "import_type": "Webpage",
                "artifact": {
                  "url": url,
                  "preserve_image": result.preserveImage,
                }
              })
          .toList();

      await _performImport(imports, collectionId);
    }
  }

  Future<void> _importTextFile(String collectionId) async {
    setState(() => _isLoading = true);

    await ImportExportService.importTextFiles(
      onBatchReady: (batch) async {
        await AppStateScope.of(
          context,
        ).importDocuments(batch, collectionId: collectionId);
      },
      onError: (e) {
        if (mounted) {
          _handleImportError(e);
        }
      },
      onSuccess: (count) {
        if (mounted && count > 0) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text('Started $count import batches.')),
          );
        }
      },
    );

    if (mounted) setState(() => _isLoading = false);
  }

  Future<void> _importDatabase(String collectionId) async {
    final result = await showDialog<Map<String, dynamic>>(
      context: context,
      builder: (context) => const ImportDatabaseDialog(),
    );

    if (result != null) {
      if (!mounted) return;
      final imports = [
        {"import_type": "RelationshipDatabase", "artifact": result},
      ];
      await _performImport(imports, collectionId);
    }
  }

  Future<void> _exportCollection(String collectionId) async {
    final appState = AppStateScope.of(context);
    final documents = appState.documentsByCollectionId[collectionId] ?? [];

    if (documents.isEmpty) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
            content: Text('No documents to export in this collection.'),
          ),
        );
      }
      return;
    }

    setState(() => _isLoading = true);

    await ImportExportService.exportCollection(
      documents: documents,
      collectionTitle: widget.collection.title,
      fetchDocumentContent: (docId) async {
        final chunks = await appState.documents.getDocument(
          appState.dio,
          docId,
        );
        return chunks.map((c) => c.content).join('');
      },
      onComplete: (success, failed) {
        if (mounted) {
          if (success > 0) {
            ScaffoldMessenger.of(context).showSnackBar(
              SnackBar(
                content: Text('Exported $success documents. Failed: $failed'),
              ),
            );
          } else {
            ScaffoldMessenger.of(context).showSnackBar(
              SnackBar(
                content: Text(
                  'No documents were successfully prepared for export. Failed: $failed',
                ),
              ),
            );
          }
        }
      },
      onError: (e) {
        if (mounted) {
          ScaffoldMessenger.of(
            context,
          ).showSnackBar(SnackBar(content: Text('Error during export: $e')));
        }
      },
    );

    if (mounted) setState(() => _isLoading = false);
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

  void _showCollectionMenu(BuildContext context, Offset position) {
    final appState = AppStateScope.of(context);
    showMenu(
      context: context,
      position: RelativeRect.fromLTRB(
        position.dx,
        position.dy,
        position.dx,
        position.dy,
      ),
      items: [
        const PopupMenuItem(
          value: 'create_document',
          child: Text('New Document'),
        ),
        const PopupMenuItem(value: 'rename', child: Text('Rename Collection')),
        const PopupMenuItem(value: 'delete', child: Text('Delete Collection')),
        const PopupMenuItem(value: 'import', child: Text('Import')),
        const PopupMenuItem(value: 'export', child: Text('Export')),
      ],
    ).then((value) async {
      if (value == 'delete') {
        appState.deleteCollection(widget.collection.id);
      } else if (value == 'rename') {
        if (!mounted) return;
        final title = await showNameDialog(
          context,
          'Rename Collection',
          initialValue: widget.collection.title,
        );
        if (title != null && title.isNotEmpty) {
          appState.renameCollection(widget.collection.id, title);
        }
      } else if (value == 'create_document') {
        appState.createLocalDocument(widget.collection.id);
        if (mounted) {
          setState(() {
            _isExpanded = true;
          });
        }
      } else if (value == 'import') {
        _showImportOptionsDialog(widget.collection.id);
      } else if (value == 'export') {
        _exportCollection(widget.collection.id);
      }
    });
  }

  void _toggleExpansion() {
    setState(() {
      _isExpanded = !_isExpanded;
    });
    if (_isExpanded) {
      final appState = AppStateScope.of(context);
      appState.setActiveObject(
        ActiveObjectType.collection,
        widget.collection.id,
      );
      appState.fetchDocumentsForCollection(widget.collection.id);
    }
  }

  Widget createCollectionListTile(AppState appState) {
    return Container(
      color: _tileFocusNode.hasFocus
          ? Theme.of(context).colorScheme.secondaryContainer
          : null,
      child: ListTile(
        autofocus: widget.autofocus,
        focusNode: _tileFocusNode,
        onTap: () {
          // _tileFocusNode.requestFocus();
          _toggleExpansion();
        },
        title: Tooltip(
          preferBelow: false,
          richMessage: WidgetSpan(
            child: Column(
              children: [
                Text('Created: ${widget.collection.createdAt}'),
                Text('Modified: ${widget.collection.lastModified}'),
              ],
            ),
          ),
          child: Row(
            children: [
              Icon(
                _isExpanded ? Icons.expand_more : Icons.chevron_right,
                size: 20,
                color: Theme.of(context).iconTheme.color,
              ),
              const SizedBox(width: 8),
              Expanded(child: Text(widget.collection.title)),
              Builder(
                builder: (context) {
                  return IconButton(
                    focusNode: _focusCollectionListTiles,
                    icon: const Icon(Icons.more_vert, size: 16),
                    onPressed: () {
                      final renderBox = context.findRenderObject() as RenderBox;
                      final offset = renderBox.localToGlobal(Offset.zero);
                      _showCollectionMenu(
                        context,
                        offset + Offset(0, renderBox.size.height),
                      );
                    },
                  );
                },
              ),
            ],
          ),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final appState = AppStateScope.of(context);
    final documents = appState.getDocumentMetadatasList(widget.collection.id);

    return DragTarget<DocumentMetadata>(
      onWillAccept: (data) =>
          data != null && data.collectionMetadataId != widget.collection.id,
      onAccept: (data) {
        appState.moveDocument(data.id, widget.collection.id);
      },
      builder: (context, candidateData, rejectedData) {
        return Column(
          children: [
            GestureDetector(
              onSecondaryTapDown: (details) {
                appState.setActiveObject(
                  ActiveObjectType.collection,
                  widget.collection.id,
                );
                _showCollectionMenu(context, details.globalPosition);
              },
              child: Container(
                color: candidateData.isNotEmpty
                    ? Theme.of(context).colorScheme.primaryContainer
                    : null,
                child: createCollectionListTile(appState),
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
                      childWhenDragging: Opacity(
                        opacity: 0.5,
                        child: DocumentTile(doc: doc),
                      ),
                      child: DocumentTile(doc: doc),
                    );
                  }).toList(),
                ),
              ),
          ],
        );
      },
    );
  }
}

class DocumentTile extends StatefulWidget {
  final DocumentMetadata doc;
  const DocumentTile({super.key, required this.doc});

  @override
  State<DocumentTile> createState() => _DocumentTileState();
}

class _DocumentTileState extends State<DocumentTile> {
  late final FocusNode _focusNode;
  final FocusNode _menuFocusNode = FocusNode(skipTraversal: true);

  @override
  void initState() {
    super.initState();
    _focusNode = FocusNode(onKeyEvent: _handleKeyEvent);
    _focusNode.addListener(_onFocusChange);
  }

  KeyEventResult _handleKeyEvent(FocusNode node, KeyEvent event) {
    final appState = AppStateScope.of(context);

    if (!appState.keyBindings.isVimEnabled) {
      return KeyEventResult.ignored;
    }

    final (action, _) = appState.keyBindings.resolve(event);

    if (action != null) {
      if (action == AppAction.cursorMoveDown) {
        FocusScope.of(context).focusInDirection(TraversalDirection.down);
        return KeyEventResult.handled;
      } else if (action == AppAction.cursorMoveUp) {
        FocusScope.of(context).focusInDirection(TraversalDirection.up);
        return KeyEventResult.handled;
      }
    }

    return KeyEventResult.ignored;
  }

  void _onFocusChange() {
    if (mounted) setState(() {});
  }

  @override
  void dispose() {
    _focusNode.removeListener(_onFocusChange);
    _focusNode.dispose();
    _menuFocusNode.dispose();
    super.dispose();
  }

  void _showMenu(Offset position) {
    final appState = AppStateScope.of(context);
    showMenu(
      context: context,
      position: RelativeRect.fromLTRB(
        position.dx,
        position.dy,
        position.dx,
        position.dy,
      ),
      items: [
        const PopupMenuItem(value: 'rename', child: Text('Rename')),
        const PopupMenuItem(value: 'delete', child: Text('Delete')),
      ],
    ).then((value) async {
      if (value == 'delete') {
        appState.deleteDocument(widget.doc.id);
      } else if (value == 'rename') {
        final title = await showNameDialog(
          context,
          'Rename Document',
          initialValue: widget.doc.title,
        );
        if (title != null && title.isNotEmpty) {
          appState.renameDocument(widget.doc.id, title, appState.pollTasks);
        }
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final appState = AppStateScope.of(context);

    return GestureDetector(
      onSecondaryTapDown: (details) {
        // _focusNode.requestFocus();
        appState.setActiveObject(ActiveObjectType.document, widget.doc.id);
        _showMenu(details.globalPosition);
      },
      child: Container(
        color: _focusNode.hasFocus
            ? Theme.of(context).colorScheme.secondaryContainer
            : null,
        child: ListTile(
          focusNode: _focusNode,
          title: Text(widget.doc.title),
          leading: const Icon(Icons.article),
          selected:
              appState.activeObject.type == ActiveObjectType.document &&
              appState.activeObject.id == widget.doc.id,
          onTap: () {
            // _focusNode.requestFocus();
            appState.openDocument(widget.doc.id);
          },
          trailing: Builder(
            builder: (context) {
              return IconButton(
                focusNode: _menuFocusNode,
                icon: const Icon(Icons.more_vert, size: 16),
                onPressed: () {
                  final renderBox = context.findRenderObject() as RenderBox;
                  final offset = renderBox.localToGlobal(Offset.zero);
                  _showMenu(offset + Offset(0, renderBox.size.height));
                },
              );
            },
          ),
        ),
      ),
    );
  }
}
