import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:notes/actions.dart';
import 'package:notes/services/key_mapping.dart';
import 'package:notes/state/app_state_scope.dart';

enum EditorMode { normal, insert, visual, visualLine }

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

class _EditorInputHandlerState extends State<EditorInputHandler> {
  EditorMode _mode =
      EditorMode.insert; // Default to insert (conventional) until config loads
  final UndoHistoryController _undoController = UndoHistoryController();

  // Selection anchor for Visual Mode
  int? _visualStart;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final appState = AppStateScope.of(context);

    // Initialize mode based on configuration
    if (appState.keyBindings.isVimEnabled) {
      if (_mode == EditorMode.insert && !_initialized) {
        _mode = EditorMode.normal;
      }
    }
    _initialized = true;
  }

  bool _initialized = false;

  void _switchMode(EditorMode newMode) {
    setState(() {
      _mode = newMode;
      if (newMode == EditorMode.visual) {
        _visualStart = widget.controller.selection.baseOffset;
      } else if (newMode == EditorMode.visualLine) {
        _visualStart = widget.controller.selection.baseOffset;
        _selectLine();
      } else {
        _visualStart = null;
        // When exiting visual, collapse selection?
        if ((_mode == EditorMode.visual || _mode == EditorMode.visualLine) &&
            newMode == EditorMode.normal) {
          widget.controller.selection = TextSelection.collapsed(
            offset: widget.controller.selection.baseOffset,
          );
        }
      }
    });
  }

  KeyContext _getCurrentContext() {
    switch (_mode) {
      case EditorMode.normal:
        return KeyContext.editorNormal;
      case EditorMode.insert:
        return KeyContext.editorInsert;
      case EditorMode.visual:
      case EditorMode.visualLine:
        return KeyContext.editorVisual;
    }
  }

  void _handleAction(AppAction action) {
    final text = widget.controller.text;
    final selection = widget.controller.selection;

    switch (action) {
      // --- Modes ---
      case AppAction.enterInsertMode:
        _switchMode(EditorMode.insert);
        break;
      case AppAction.enterVisualMode:
        _switchMode(EditorMode.visual);
        break;
      case AppAction.enterVisualLineMode:
        _switchMode(EditorMode.visualLine);
        break;
      case AppAction.exitInsertMode:
      case AppAction.exitVisualMode:
        _switchMode(EditorMode.normal);
        // Clear selection on exit
        widget.controller.selection = TextSelection.collapsed(
          offset: selection.baseOffset,
        );
        break;

      // --- Navigation ---
      case AppAction.cursorMoveLeft:
        _moveCursor(-1);
        break;
      case AppAction.cursorMoveRight:
        _moveCursor(1);
        break;
      case AppAction.cursorMoveUp:
        _moveVertical(-1);
        break;
      case AppAction.cursorMoveDown:
        _moveVertical(1);
        break;
      case AppAction.moveWordForward:
        _moveWord(1);
        break;
      case AppAction.moveWordBackward:
        _moveWord(-1);
        break;

      // --- Editing ---
      case AppAction.deleteLeft: // Backspace
        _deleteText(-1);
        break;
      case AppAction.deleteRight: // Delete
      case AppAction.deleteSelection: // 'x' or 'd'
        _deleteText(1); // or delete selection
        break;
      case AppAction.deleteLine:
        _deleteLine();
        break;

      case AppAction.yank:
      case AppAction.yankSelection:
        _yankSelection();
        break;
      case AppAction.yankLine:
        _yankLine();
        break;

      case AppAction.undo:
        _undoController.undo();
        break;
      case AppAction.redo:
        _undoController.redo();
        break;

      case AppAction.saveDocument:
        performAction(context, AppAction.saveDocument);
        break;

      case AppAction.gotoBeginningOfDocument:
        _moveCursor(-widget.controller.text.length); // Move to 0
        break;
      case AppAction.gotoEndOfDocument:
        _moveCursor(widget.controller.text.length);
        break;

      case AppAction.scrollDownHalfPage:
        _scrollPage(0.5);
        break;
      case AppAction.scrollUpHalfPage:
        _scrollPage(-0.5);
        break;

      default:
        break;
    }
  }

  void _scrollPage(double factor) {
    if (!widget.scrollController.hasClients) return;
    final position = widget.scrollController.position;
    final pageSize = position.viewportDimension;
    final delta = pageSize * factor;
    final newOffset = (position.pixels + delta).clamp(
      position.minScrollExtent,
      position.maxScrollExtent,
    );
    widget.scrollController.jumpTo(newOffset);

    // Also move cursor roughly?
    // For now just scroll view. Vim usually moves cursor too.
    // Implementing cursor movement relative to scroll view requires RenderObject access
    // or complex calculation, skipping for now.
  }

  void _selectLine() {
    final text = widget.controller.text;
    final selection = widget.controller.selection;
    if (text.isEmpty) return;

    final start = selection.baseOffset <= 0
        ? -1
        : text.lastIndexOf('\n', selection.baseOffset - 1);
    final end = text.indexOf('\n', selection.baseOffset);

    widget.controller.selection = TextSelection(
      baseOffset: start == -1 ? 0 : start + 1,
      extentOffset: end == -1 ? text.length : end + 1,
    );
  }

  void _moveWord(int direction) {
    // Simple word movement implementation
    final text = widget.controller.text;
    final current = widget.controller.selection.baseOffset;
    if (text.isEmpty) return;

    int newOffset = current;
    if (direction > 0) {
      // Forward: Find next whitespace or punctuation
      final nextSpace = text.indexOf(
        RegExp(r'\s|\p{P}', unicode: true),
        current + 1,
      );
      if (nextSpace != -1) {
        newOffset = nextSpace + 1; // Basic jump
      } else {
        newOffset = text.length;
      }
    } else {
      // Backward: Find previous whitespace or punctuation
      final prevSpaceStartIndex = current - 1;
      final prevSpace = prevSpaceStartIndex < 0
          ? -1
          : text.lastIndexOf(
              RegExp(r'\s|\p{P}', unicode: true),
              prevSpaceStartIndex,
            );
      if (prevSpace != -1) {
        newOffset = prevSpace; // Basic jump to start of word
      } else {
        newOffset = 0;
      }
    }
    _moveCursor(newOffset - current);
  }

  void _deleteLine() {
    final text = widget.controller.text;
    final selection = widget.controller.selection;
    if (text.isEmpty) return;

    final start = selection.baseOffset <= 0
        ? -1
        : text.lastIndexOf('\n', selection.baseOffset - 1);
    final end = text.indexOf('\n', selection.baseOffset);

    final deleteStart = start == -1 ? 0 : start + 1;
    final deleteEnd = end == -1 ? text.length : end + 1;

    final newText = text.replaceRange(deleteStart, deleteEnd, '');
    widget.controller.value = TextEditingValue(
      text: newText,
      selection: TextSelection.collapsed(offset: deleteStart),
    );
    widget.onChanged(newText);
  }

  Future<void> _yankSelection() async {
    final selection = widget.controller.selection;
    final text = widget.controller.text;
    if (selection.isCollapsed) return;

    final selectedText = text.substring(selection.start, selection.end);
    await Clipboard.setData(ClipboardData(text: selectedText));

    if (_mode == EditorMode.visual || _mode == EditorMode.visualLine) {
      _switchMode(EditorMode.normal);
      widget.controller.selection = TextSelection.collapsed(
        offset: selection.baseOffset,
      );
    }
  }

  Future<void> _yankLine() async {
    final text = widget.controller.text;
    final selection = widget.controller.selection;
    if (text.isEmpty) return;

    final start = selection.baseOffset <= 0
        ? -1
        : text.lastIndexOf('\n', selection.baseOffset - 1);
    final end = text.indexOf('\n', selection.baseOffset);

    final yankStart = start == -1 ? 0 : start + 1;
    final yankEnd = end == -1 ? text.length : end + 1;

    final lineText = text.substring(yankStart, yankEnd);
    // Include newline in yank if it exists?
    // Usually 'yy' copies the line including the newline character.
    // Our 'end' search finds the next newline.
    // substring(start, end) excludes the newline at 'end'.
    // If we want to copy the WHOLE line including the newline at the end:
    // If it's the last line, it might not have a newline.

    String textToCopy = lineText;
    // Check if there is a newline at the end
    if (yankEnd <= text.length &&
        end != -1 &&
        text.substring(yankEnd - 1, yankEnd) == '\n') {
      // It's already included?
      // Wait, end is index of '\n'. substring(start, end) excludes char at end.
      // So lineText does NOT include \n.
      // We should include it for 'yy'.
      if (end != -1) {
        textToCopy = text.substring(yankStart, end + 1);
      }
    } else if (end != -1) {
      // If we are not at EOF, append newline
      textToCopy = text.substring(yankStart, end + 1);
    }

    await Clipboard.setData(ClipboardData(text: textToCopy));

    // Flash or feedback could go here
    if (_mode == EditorMode.visual || _mode == EditorMode.visualLine) {
      _switchMode(EditorMode.normal);
      widget.controller.selection = TextSelection.collapsed(
        offset: selection.baseOffset,
      );
    }
  }

  void _moveCursor(int delta) {
    final text = widget.controller.text;
    final selection = widget.controller.selection;
    var newOffset = selection.baseOffset + delta;

    if (newOffset < 0) newOffset = 0;
    if (newOffset > text.length) newOffset = text.length;

    if (_mode == EditorMode.visual && _visualStart != null) {
      // Extend selection
      widget.controller.selection = TextSelection(
        baseOffset: _visualStart!,
        extentOffset: newOffset,
      );
    } else {
      // Move caret
      widget.controller.selection = TextSelection.collapsed(offset: newOffset);
    }
  }

  void _moveVertical(int delta) {
    // Vertical movement in TextField is tricky without RenderObject access.
    // For now, let's defer to default behavior if in Insert mode?
    // Or approximate by finding newlines.

    // Simple approximation: Jump to next/prev newline
    // This is NOT accurate column preservation, but a start.
    final text = widget.controller.text;
    final current = widget.controller.selection.baseOffset;

    if (delta > 0) {
      // Down
      final nextNewline = text.indexOf('\n', current);
      if (nextNewline != -1) {
        _moveCursor(nextNewline - current + 1); // +1 to pass the \n
      }
    } else {
      // Up
      if (current <= 0) return;

      final prevNewline = text.lastIndexOf('\n', current - 1);
      if (prevNewline != -1) {
        // Go to the newline before that
        final prevPrev = prevNewline <= 0
            ? -1
            : text.lastIndexOf('\n', prevNewline - 1);
        final targetOffset = prevPrev == -1 ? 0 : prevPrev + 1;
        _moveCursor(targetOffset - current);
      } else {
        _moveCursor(-current); // Go to start
      }
    }
  }

  void _deleteText(int direction) {
    final selection = widget.controller.selection;
    final text = widget.controller.text;

    if (!selection.isCollapsed) {
      // Delete selection
      final newText = text.replaceRange(selection.start, selection.end, '');
      widget.controller.value = TextEditingValue(
        text: newText,
        selection: TextSelection.collapsed(offset: selection.start),
      );
      widget.onChanged(newText);
      return;
    }

    // Delete char
    if (direction < 0) {
      // Backspace
      if (selection.baseOffset > 0) {
        final newText = text.replaceRange(
          selection.baseOffset - 1,
          selection.baseOffset,
          '',
        );
        widget.controller.value = TextEditingValue(
          text: newText,
          selection: TextSelection.collapsed(offset: selection.baseOffset - 1),
        );
        widget.onChanged(newText);
      }
    } else {
      // Delete
      if (selection.baseOffset < text.length) {
        final newText = text.replaceRange(
          selection.baseOffset,
          selection.baseOffset + 1,
          '',
        );
        widget.controller.value = TextEditingValue(
          text: newText,
          selection: TextSelection.collapsed(offset: selection.baseOffset),
        );
        widget.onChanged(newText);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final appState = AppStateScope.of(context);

    // Visual indicator of mode (Optional, but helpful)
    final modeColor = _mode == EditorMode.normal
        ? Colors.green.withOpacity(0.1)
        : _mode == EditorMode.visual
        ? Colors.blue.withOpacity(0.1)
        : Colors.transparent;

    // Determine the cursor size by mode
    double cursorWidth = 2.0;
    if (_mode != EditorMode.insert) {
      cursorWidth = 12.0;
    }

    return Focus(
      onKeyEvent: (node, event) {
        if (event is! KeyDownEvent && event is! KeyRepeatEvent) {
          return KeyEventResult.ignored;
        }

        // 1. Resolve Action (Editor Context)
        final context = _getCurrentContext();
        final action = appState.keyBindings.resolve(context, event);

        if (action != null) {
          _handleAction(action);
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
        if (_mode != EditorMode.insert) {
          return KeyEventResult.handled;
        }

        // 4. In Insert mode, let it bubble to TextField
        return KeyEventResult.ignored;
      },
      child: Container(
        color: modeColor,
        child: TextField(
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
