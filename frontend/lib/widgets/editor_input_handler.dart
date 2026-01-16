import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:notes/actions/editor.dart';
import 'package:notes/services/key_mapping.dart';
import 'package:notes/state/app_state_scope.dart';

class EditorInputHandler extends StatefulWidget {
  final TextEditingController controller;
  final FocusNode focusNode;
  final ScrollController scrollController;
  final Function(String) onChanged;
  final VoidCallback onSave;

  const EditorInputHandler({
    super.key,
    required this.controller,
    required this.focusNode,
    required this.scrollController,
    required this.onChanged,
    required this.onSave,
  });

  @override
  State<EditorInputHandler> createState() => _EditorInputHandlerState();
}

class _EditorInputHandlerState extends State<EditorInputHandler>
    with EditorShortcuts {
  final UndoHistoryController _undoController = UndoHistoryController();

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final appState = AppStateScope.of(context);

    // Initialize mode based on configuration
    if (appState.keyBindings.isVimEnabled) {
      if (mode == EditorMode.insert && !_initialized) {
        mode = EditorMode.normal;
      }
    }
    _initialized = true;
  }

  bool _initialized = false;

  @override
  Widget build(BuildContext context) {
    final appState = AppStateScope.of(context);

    // Visual indicator of mode (Optional, but helpful)
    final modeColor = mode == EditorMode.normal
        ? Colors.green.withOpacity(0.1)
        : mode == EditorMode.visual
        ? Colors.blue.withOpacity(0.1)
        : Colors.transparent;

    // Determine the cursor size by mode
    double cursorWidth = 2.0;
    if (mode != EditorMode.insert) {
      cursorWidth = 12.0;
    }

    return Focus(
      onKeyEvent: (node, event) {
        if (event is! KeyDownEvent && event is! KeyRepeatEvent) {
          return KeyEventResult.ignored;
        }

        // 1. Resolve Action (Editor Context)
        final context = getCurrentContext();
        final action = appState.keyBindings.resolve(context, event);

        if (action != null) {
          handleAction(
            action,
            widget.scrollController,
            widget.controller,
            _undoController,
          );
          return KeyEventResult.handled;
        }

        // 2. Check if it's a Global Action
        // If it is, we MUST ignore it so it bubbles up to GlobalKeyHandler
        final globalAction = appState.keyBindings.resolve(
          KeyContext.global,
          event,
        );
        if (globalAction != null) {
          return KeyEventResult.ignored;
        }

        // 3. If no action, and NOT in Insert mode, block the key
        // (so 'j' doesn't type 'j' in Normal mode)
        if (mode != EditorMode.insert) {
          return KeyEventResult.handled;
        }

        // 4. In Insert mode, let it bubble to TextField
        return KeyEventResult.ignored;
      },
      child: Container(
        color: modeColor,
        child: TextField(
          // readOnly: mode != EditorMode.insert,
          showCursor: true,
          cursorWidth: cursorWidth,
          controller: widget.controller,
          undoController: _undoController,
          scrollController: widget.scrollController,
          focusNode: widget.focusNode,
          maxLines: null,
          expands: true,
          style: Theme.of(context).textTheme.bodyLarge?.copyWith(height: 1.5),
          decoration: const InputDecoration(
            border: InputBorder.none,
            contentPadding: EdgeInsets.all(16),
            hintText: 'Start typing...',
          ),
          onChanged: widget.onChanged,
        ),
      ),
    );
  }
}
