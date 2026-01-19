import 'package:flutter/material.dart';
import 'package:notes/inputs/conventional_editor_input_handler.dart';
import 'package:notes/inputs/vim_editor_input_handler.dart';
import 'package:notes/state/app_state_scope.dart';

class EditorInputHandler extends StatelessWidget {
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
  Widget build(BuildContext context) {
    final appState = AppStateScope.of(context);
    final isVimEnabled = appState.keyBindings.isVimEnabled;

    if (isVimEnabled) {
      return VimEditorInputHandler(
        controller: controller,
        focusNode: focusNode,
        scrollController: scrollController,
        onChanged: onChanged,
        onSave: onSave,
      );
    } else {
      return ConventionalEditorInputHandler(
        controller: controller,
        focusNode: focusNode,
        scrollController: scrollController,
        onChanged: onChanged,
        onSave: onSave,
      );
    }
  }
}
