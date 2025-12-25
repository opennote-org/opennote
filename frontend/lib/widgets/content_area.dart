import 'package:flutter/material.dart';
import 'package:flutter_markdown_plus/flutter_markdown_plus.dart';
import 'package:notes/show.dart';
import 'package:notes/state/app_state_scope.dart';

class ContentArea extends StatelessWidget {
  const ContentArea({super.key});

  @override
  Widget build(BuildContext context) {
    final appState = AppStateScope.of(context);
    final openDocIds = appState.openDocumentIds;
    final activeDocId = appState.activeDocumentId;

    if (openDocIds.isEmpty) {
      return Focus(
        autofocus: true,
        child: Builder(
          builder: (context) {
            return GestureDetector(
              behavior: HitTestBehavior.translucent,
              onTap: () => FocusScope.of(context).requestFocus(Focus.of(context)),
              child: Center(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    _buildShortcutRow(showSearchPopup, context, 'Search', 'Ctrl + P', Icons.search),
                    _buildShortcutRow(showConfigurationPopup, context, 'Configurations', 'Ctrl + C', Icons.settings),
                  ],
                ),
              ),
            );
          },
        ),
      );
    }

    return Column(
      children: [
        // Tab Bar
        SizedBox(
          height: 48,
          child: ListView.builder(
            scrollDirection: Axis.horizontal,
            itemCount: openDocIds.length,
            itemBuilder: (context, index) {
              final docId = openDocIds[index];
              final doc = appState.documentById[docId];
              final isActive = docId == activeDocId;

              return InkWell(
                onTap: () => appState.setActiveDocument(docId),
                child: Container(
                  width: 150, // Fixed width for tabs
                  padding: const EdgeInsets.symmetric(horizontal: 8),
                  alignment: Alignment.center,
                  decoration: BoxDecoration(
                    color: isActive ? Theme.of(context).colorScheme.surface : Theme.of(context).colorScheme.surfaceContainerHighest,
                    border: Border(
                      right: BorderSide(color: Theme.of(context).dividerColor),
                      top: isActive ? BorderSide(color: Theme.of(context).primaryColor, width: 2) : BorderSide.none,
                    ),
                  ),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Expanded(child: Text(doc?.title ?? 'Loading...', overflow: TextOverflow.ellipsis, maxLines: 1)),
                      const SizedBox(width: 4),
                      InkWell(onTap: () => appState.closeDocument(docId), child: const Icon(Icons.close, size: 16)),
                    ],
                  ),
                ),
              );
            },
          ),
        ),
        // Content
        Expanded(
          child: activeDocId != null ? DocumentEditor(key: ValueKey(activeDocId), documentId: activeDocId) : const SizedBox(),
        ),
      ],
    );
  }

  Widget _buildShortcutRow(
    void Function(BuildContext context) popupFunction,
    BuildContext context,
    String label,
    String shortcut,
    IconData icon,
  ) {
    void _popupFunction() => popupFunction(context);

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          GestureDetector(
            onTap: _popupFunction,
            child: Row(
              spacing: 12,
              children: [
                Icon(icon, size: 20, color: Theme.of(context).colorScheme.secondary),
                Text(label, style: Theme.of(context).textTheme.bodyLarge?.copyWith(color: Theme.of(context).colorScheme.secondary)),
                Text(
                  shortcut,
                  style: Theme.of(
                    context,
                  ).textTheme.bodyLarge?.copyWith(fontWeight: FontWeight.bold, color: Theme.of(context).colorScheme.onSurface),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class DocumentEditor extends StatefulWidget {
  final String documentId;
  const DocumentEditor({super.key, required this.documentId});

  @override
  State<DocumentEditor> createState() => _DocumentEditorState();
}

class _DocumentEditorState extends State<DocumentEditor> {
  late TextEditingController _controller;
  final FocusNode _focusNode = FocusNode();

  @override
  void initState() {
    super.initState();
    _controller = TextEditingController();
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
    if (_controller.text != content && content.isNotEmpty && _controller.text.isEmpty) {
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
          final sortedParts = List<String>.from(parts)..sort((a, b) => b.length.compareTo(a.length));
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

        // Calculate selection
        final selection = TextSelection(baseOffset: index, extentOffset: index + length);

        _controller.selection = selection;

        // Ensure focus and scroll
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (mounted) {
            _focusNode.requestFocus();
          }
        });
      }
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    _focusNode.dispose();
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
                child: TextField(
                  controller: _controller,
                  focusNode: _focusNode,
                  maxLines: null,
                  expands: true,
                  decoration: const InputDecoration(
                    border: InputBorder.none,
                    contentPadding: EdgeInsets.all(16),
                    hintText: 'Start typing...',
                  ),
                  onChanged: (value) {
                    appState.updateDocumentDraft(widget.documentId, value);
                    // We need setState because we want the markdown view updated in real-time
                    setState(() {});
                  },
                ),
              ),
              const VerticalDivider(width: 1),
              Expanded(child: Markdown(data: _controller.text)),
            ],
          ),
        ),
      ],
    );
  }
}
