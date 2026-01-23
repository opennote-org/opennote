import 'package:flutter/material.dart';
import 'package:flutter_markdown_plus/flutter_markdown_plus.dart';
import 'package:notes/inputs/editor_input_handler.dart';
import 'package:notes/state/app_state_scope.dart';

class DocumentEditor extends StatefulWidget {
  final String documentId;
  const DocumentEditor({super.key, required this.documentId});

  @override
  State<DocumentEditor> createState() => _DocumentEditorState();
}

class _DocumentEditorState extends State<DocumentEditor> {
  late TextEditingController _controller;
  final FocusNode _focusNode = FocusNode();
  late ScrollController _editorScrollController;
  late ScrollController _previewScrollController;
  bool _isScrolling = false;

  @override
  void initState() {
    super.initState();
    _controller = TextEditingController();
    _editorScrollController = ScrollController();
    _previewScrollController = ScrollController();

    _editorScrollController.addListener(() {
      if (_isScrolling) return;
      _isScrolling = true;
      if (_previewScrollController.hasClients &&
          _editorScrollController.hasClients) {
        double percentage = 0.0;
        if (_editorScrollController.position.maxScrollExtent > 0) {
          percentage =
              _editorScrollController.offset /
              _editorScrollController.position.maxScrollExtent;
        }

        if (_previewScrollController.position.maxScrollExtent > 0) {
          _previewScrollController.jumpTo(
            percentage * _previewScrollController.position.maxScrollExtent,
          );
        }
      }
      _isScrolling = false;
    });

    _previewScrollController.addListener(() {
      if (_isScrolling) return;
      _isScrolling = true;
      if (_editorScrollController.hasClients &&
          _previewScrollController.hasClients) {
        double percentage = 0.0;
        if (_previewScrollController.position.maxScrollExtent > 0) {
          percentage =
              _previewScrollController.offset /
              _previewScrollController.position.maxScrollExtent;
        }

        if (_editorScrollController.position.maxScrollExtent > 0) {
          _editorScrollController.jumpTo(
            percentage * _editorScrollController.position.maxScrollExtent,
          );
        }
      }
      _isScrolling = false;
    });
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _updateContent();
  }

  @override
  void didUpdateWidget(DocumentEditor oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.documentId != widget.documentId) {
      _updateContent();
    }
  }

  void _updateContent() {
    final appState = AppStateScope.of(context);
    final content = appState.documentContentCache[widget.documentId] ?? '';
    bool contentUpdated = false;

    // Only update if drastically different to avoid cursor jump?
    // Or simpler: only update if controller is empty (initial load).
    // If user is typing, we shouldn't overwrite unless it's a remote update.
    // For now, let's assume single user editing locally.
    if (_controller.text != content &&
        content.isNotEmpty &&
        _controller.text.isEmpty) {
      _controller.text = content;
      contentUpdated = true;
    } else if (content.isNotEmpty && _controller.text != content) {
      // Logic for remote update conflicts could go here.
      // For now, respect local edits over cache unless cache is empty?
      // Actually, if we switch tabs, we reload from cache.
      // So if cache is updated by us, it's fine.
      _controller.text = content;
      contentUpdated = true;
    }

    if (contentUpdated || content.isNotEmpty) {
      _checkHighlight();
    }
  }

  void _checkHighlight() {
    final appState = AppStateScope.of(context);
    final highlight = appState.searchHighlights[widget.documentId];
    if (highlight != null && _controller.text.isNotEmpty) {
      final highlightText = highlight.text;
      int index = -1;
      int length = highlightText.length;

      // Try chunk offset
      if (highlight.chunkId != null) {
        final offsets = appState.documentChunkOffsets[widget.documentId];
        if (offsets != null && offsets.containsKey(highlight.chunkId)) {
          index = offsets[highlight.chunkId]!;
        }
      }

      if (index == -1) {
        index = _controller.text.indexOf(highlightText);
      }

      // Fallback 1: Try matching the first 50 characters
      // This handles cases where the chunk is long and may have minor tail differences
      if (index == -1 && highlightText.length > 50) {
        final shortHighlight = highlightText.substring(0, 50);
        index = _controller.text.indexOf(shortHighlight);
        if (index != -1) {
          length = 50;
        }
      }

      // Fallback 2: Try matching segments to avoid newline mismatch (\r\n vs \n)
      if (index == -1) {
        final parts = highlightText.split(RegExp(r'[\r\n]+'));
        String? bestPart;
        // Find the first significant part
        for (final part in parts) {
          if (part.length > 15) {
            bestPart = part;
            break;
          }
        }
        // If no long part found, take the longest one available
        if (bestPart == null && parts.isNotEmpty) {
          // Create a copy to sort
          final sortedParts = List<String>.from(parts)
            ..sort((a, b) => b.length.compareTo(a.length));
          if (sortedParts.isNotEmpty) bestPart = sortedParts.first;
        }

        if (bestPart != null && bestPart.isNotEmpty) {
          index = _controller.text.indexOf(bestPart);
          if (index != -1) {
            length = bestPart.length;
          }
        }
      }

      if (index != -1) {
        // Clear highlight from state so we don't jump again
        appState.searchHighlights.remove(widget.documentId);

        final textLength = _controller.text.length;
        final baseOffset = index.clamp(0, textLength);
        final extentOffset = (index + length).clamp(0, textLength);
        final selection = TextSelection(
          baseOffset: baseOffset,
          extentOffset: extentOffset,
        );

        print(index);
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (!mounted) return;
          _controller.selection = selection;
          _focusNode.requestFocus();
        });
      }
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    _focusNode.dispose();
    _editorScrollController.dispose();
    _previewScrollController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final appState = AppStateScope.of(context);
    return Column(
      children: [
        // Editor
        Expanded(
          child: Row(
            children: [
              Expanded(
                child: EditorInputHandler(
                  controller: _controller,
                  scrollController: _editorScrollController,
                  focusNode: _focusNode,
                  onChanged: (value) {
                    appState.updateDocumentDraft(widget.documentId, value);
                    // We need setState because we want the markdown view updated in real-time
                    setState(() {});
                  },
                  onSave: () => appState.saveActiveDocument(),
                ),
              ),
              const VerticalDivider(width: 1),
              Expanded(
                child: Markdown(
                  data: _controller.text,
                  controller: _previewScrollController,
                  styleSheet: MarkdownStyleSheet.fromTheme(Theme.of(context))
                      .copyWith(
                        p: Theme.of(
                          context,
                        ).textTheme.bodyLarge?.copyWith(height: 1.5),
                        blockSpacing: 24.0,
                      ),
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}
