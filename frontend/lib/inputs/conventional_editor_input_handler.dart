import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:notes/actions/editor.dart';
import 'package:notes/state/app_state_scope.dart';

class ConventionalEditorInputHandler extends StatefulWidget {
  final TextEditingController controller;
  final FocusNode focusNode;
  final ScrollController scrollController;
  final Function(String) onChanged;
  final VoidCallback onSave;

  const ConventionalEditorInputHandler({
    super.key,
    required this.controller,
    required this.focusNode,
    required this.scrollController,
    required this.onChanged,
    required this.onSave,
  });

  @override
  State<ConventionalEditorInputHandler> createState() =>
      _ConventionalEditorInputHandler();
}

class _ConventionalEditorInputHandler
    extends State<ConventionalEditorInputHandler>
    with EditorActions {
  final UndoHistoryController _undoController = UndoHistoryController();

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
  }

  @override
  Widget build(BuildContext context) {
    final appState = AppStateScope.of(context);

    return Focus(
      onKeyEvent: (node, event) {
        if (event is! KeyDownEvent && event is! KeyRepeatEvent) {
          return KeyEventResult.ignored;
        }

        // No special key handling in conventional key mapping
        return KeyEventResult.ignored;
      },
      child: Container(
        color: Colors.transparent,
        child: TextField(
          showCursor: true,
          cursorWidth: 2.0,
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
