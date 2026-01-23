import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:notes/actions/editor.dart';
import 'package:notes/services/key_mapping.dart';
import 'package:notes/state/app_state_scope.dart';

class VimEditorInputHandler extends StatefulWidget {
  final TextEditingController controller;
  final FocusNode focusNode;
  final ScrollController scrollController;
  final Function(String) onChanged;
  final VoidCallback onSave;

  const VimEditorInputHandler({
    super.key,
    required this.controller,
    required this.focusNode,
    required this.scrollController,
    required this.onChanged,
    required this.onSave,
  });

  @override
  State<VimEditorInputHandler> createState() => _VimEditorInputHandlerState();
}

class _VimEditorInputHandlerState extends State<VimEditorInputHandler>
    with EditorActions {
  final UndoHistoryController _undoController = UndoHistoryController();
  bool _initialized = false;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final appState = AppStateScope.of(context);

    // Initialize mode based on configuration
    if (appState.keyBindings.isVimEnabled) {
      if (mode == KeyContext.editorInsert && !_initialized) {
        mode = KeyContext.editorNormal;
      }
    }
    _initialized = true;
  }

  @override
  Widget build(BuildContext context) {
    final appState = AppStateScope.of(context);

    // Visual indicator of mode (Optional, but helpful)
    final modeColor = mode == KeyContext.editorNormal
        ? Colors.green.withOpacity(0.1)
        : mode == KeyContext.editorVisual
        ? Colors.blue.withOpacity(0.1)
        : Colors.transparent;

    // Determine the cursor size by mode
    double cursorWidth = 2.0;
    if (mode != KeyContext.editorInsert) {
      cursorWidth = 12.0;
    }

    return Focus(
      onKeyEvent: (node, event) {
        if (event is! KeyDownEvent && event is! KeyRepeatEvent) {
          return KeyEventResult.ignored;
        }

        // Resolve Action (Editor Context)
        final (action, keyContext) = appState.keyBindings.resolve(event);

        if (action != null && action == AppAction.exitInsertMode) {
          handleAction(
            action,
            widget.scrollController,
            widget.controller,
            _undoController,
          );
          return KeyEventResult.handled;
        }

        if (action != null &&
            keyContext != KeyContext.global &&
            mode != KeyContext.editorInsert) {
          handleAction(
            action,
            widget.scrollController,
            widget.controller,
            _undoController,
          );
          return KeyEventResult.handled;
        }

        // Handle the global actions in editorNormal mode.
        // Prevent closing the tab when typing x.
        if (action != null && mode == KeyContext.editorNormal) {
          return KeyEventResult.ignored;
        }

        // Prevent typing letters outside editorInsert mode
        if (mode != KeyContext.editorInsert) {
          return KeyEventResult.handled;
        }

        // We want the global actions to perform under normal mode
        if (mode == KeyContext.editorNormal &&
            keyContext == KeyContext.global) {
          return KeyEventResult.ignored;
        }

        // In Insert mode, let it bubble to TextField
        //
        // We need to skip the global actions to prevent users from activating it
        // by typing `x` or `/` in vim mode etc.
        return KeyEventResult.skipRemainingHandlers;
      },
      child: Container(
        color: modeColor,
        child: TextField(
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
