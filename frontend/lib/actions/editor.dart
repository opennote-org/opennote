import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:notes/actions/handlers.dart';
import 'package:notes/services/key_mapping.dart';

enum EditorMode { normal, insert, visual, visualLine }

mixin EditorShortcuts<T extends StatefulWidget> on State<T> {
  EditorMode mode = EditorMode.insert;

  // Selection anchor for Visual Mode
  int? _visualStart;
  int _preferredColumn = -1;

  int _getLineStart(TextEditingController controller, int offset) {
    if (offset <= 0) return 0;
    final idx = controller.text.lastIndexOf('\n', offset - 1);
    return idx == -1 ? 0 : idx + 1;
  }

  int _getLineEnd(TextEditingController controller, int offset) {
    if (offset >= controller.text.length) return controller.text.length;
    final idx = controller.text.indexOf('\n', offset);
    return idx == -1 ? controller.text.length : idx;
  }

  void _insertNewLine(bool above, TextEditingController controller) {
    final text = controller.text;

    if (above) {
      final start = _getLineStart(controller, controller.selection.baseOffset);
      final newText = text.replaceRange(start, start, '\n');
      controller.value = TextEditingValue(
        text: newText,
        selection: TextSelection.collapsed(offset: start),
      );
    } else {
      final end = _getLineEnd(controller, controller.selection.baseOffset);
      final newText = text.replaceRange(end, end, '\n');
      controller.value = TextEditingValue(
        text: newText,
        selection: TextSelection.collapsed(offset: end + 1),
      );
    }
  }

  int _calculateCurrentColumn(TextEditingController controller) {
    final offset = controller.selection.baseOffset;
    if (offset < 0) return 0;
    final lineStart = _getLineStart(controller, offset);
    return offset - lineStart;
  }

  void switchMode(EditorMode newMode, TextEditingController controller) {
    setState(() {
      mode = newMode;
      _preferredColumn = -1;
      if (newMode == EditorMode.visual) {
        _visualStart = controller.selection.baseOffset;
      } else if (newMode == EditorMode.visualLine) {
        _visualStart = controller.selection.baseOffset;
        selectLine(controller);
      } else {
        _visualStart = null;
        // When exiting visual, collapse selection?
        if ((mode == EditorMode.visual || mode == EditorMode.visualLine) && newMode == EditorMode.normal) {
          controller.selection = TextSelection.collapsed(offset: controller.selection.baseOffset);
        }
      }
    });
  }

  KeyContext getCurrentContext() {
    switch (mode) {
      case EditorMode.normal:
        return KeyContext.editorNormal;
      case EditorMode.insert:
        return KeyContext.editorInsert;
      case EditorMode.visual:
      case EditorMode.visualLine:
        return KeyContext.editorVisual;
    }
  }

  void handleAction(
    AppAction action,
    ScrollController scrollController,
    TextEditingController textEditingController,
    UndoHistoryController undoController,
  ) {
    switch (action) {
      // --- Modes ---
      case AppAction.enterInsertMode:
        switchMode(EditorMode.insert, textEditingController);
        break;
      case AppAction.enterVisualMode:
        switchMode(EditorMode.visual, textEditingController);
        break;
      case AppAction.enterVisualLineMode:
        switchMode(EditorMode.visualLine, textEditingController);
        break;
      case AppAction.exitInsertMode:
      case AppAction.exitVisualMode:
        switchMode(EditorMode.normal, textEditingController);
        // Clear selection on exit
        textEditingController.selection = TextSelection.collapsed(offset: textEditingController.selection.baseOffset);
        break;

      // --- Navigation ---
      case AppAction.cursorMoveLeft:
        moveCursor(-1, textEditingController);
        break;
      case AppAction.cursorMoveRight:
        moveCursor(1, textEditingController);
        break;
      case AppAction.cursorMoveUp:
        moveVertical(-1, textEditingController);
        break;
      case AppAction.cursorMoveDown:
        moveVertical(1, textEditingController);
        break;
      case AppAction.moveWordForward:
        moveWord(1, textEditingController);
        break;
      case AppAction.moveWordBackward:
        moveWord(-1, textEditingController);
        break;

      case AppAction.gotoBeginningOfLine:
        {
          final start = _getLineStart(textEditingController, textEditingController.selection.baseOffset);
          moveCursor(start - textEditingController.selection.baseOffset, textEditingController);
        }
        break;
      case AppAction.gotoEndOfLine:
        {
          final end = _getLineEnd(textEditingController, textEditingController.selection.baseOffset);
          moveCursor(end - textEditingController.selection.baseOffset, textEditingController);
        }
        break;

      case AppAction.insertAtBeginningOfLine:
        {
          final start = _getLineStart(textEditingController, textEditingController.selection.baseOffset);
          moveCursor(start - textEditingController.selection.baseOffset, textEditingController);
          switchMode(EditorMode.insert, textEditingController);
        }
        break;
      case AppAction.insertAtEndOfLine:
        {
          final end = _getLineEnd(textEditingController, textEditingController.selection.baseOffset);
          moveCursor(end - textEditingController.selection.baseOffset, textEditingController);
          switchMode(EditorMode.insert, textEditingController);
        }
        break;

      case AppAction.insertOnAboveNewline:
        switchMode(EditorMode.insert, textEditingController);
        if (mounted) _insertNewLine(true, textEditingController);
        break;
      case AppAction.insertOnBelowNewline:
        switchMode(EditorMode.insert, textEditingController);
        if (mounted) _insertNewLine(false, textEditingController);
        break;

      // --- Editing ---
      case AppAction.deleteLeft: // Backspace
        deleteText(-1, textEditingController, (String newText) {});
        break;
      case AppAction.deleteRight: // Delete
      case AppAction.deleteSelection: // 'x' or 'd'
        deleteText(1, textEditingController, (String newText) {}); // or delete selection
        break;
      case AppAction.deleteLine:
        deleteLine(textEditingController, (String newText) {});
        break;

      case AppAction.yank:
      case AppAction.yankSelection:
        yankSelection(textEditingController);
        break;
      case AppAction.yankLine:
        yankLine(textEditingController);
        break;

      case AppAction.undo:
        undoController.undo();
        break;
      case AppAction.redo:
        undoController.redo();
        break;

      case AppAction.saveDocument:
        performAction(context, AppAction.saveDocument);
        break;

      case AppAction.gotoBeginningOfDocument:
        moveCursor(-textEditingController.text.length, textEditingController); // Move to 0
        if (scrollController.hasClients) {
          scrollController.jumpTo(scrollController.position.minScrollExtent);
        }
        break;
      case AppAction.gotoEndOfDocument:
        moveCursor(textEditingController.text.length, textEditingController);
        if (scrollController.hasClients) {
          scrollController.jumpTo(scrollController.position.maxScrollExtent);
        }
        break;

      case AppAction.scrollDownHalfPage:
        scrollPage(0.5, scrollController, textEditingController);
        break;
      case AppAction.scrollUpHalfPage:
        scrollPage(-0.5, scrollController, textEditingController);
        break;

      default:
        break;
    }
  }

  void scrollPage(double factor, ScrollController scrollController, TextEditingController textController) {
    if (!scrollController.hasClients) return;
    final delta = scrollController.position.viewportDimension * factor;
    final newOffset = (scrollController.position.pixels + delta).clamp(
      scrollController.position.minScrollExtent,
      scrollController.position.maxScrollExtent,
    );

    final style = Theme.of(context).textTheme.bodyLarge?.copyWith(height: 1.5);
    final fontSize = style?.fontSize ?? 16.0;
    final lineHeight = fontSize * (style?.height ?? 1.5);

    // Calculate lines to move
    final linesToMove = (delta / lineHeight).round();
    moveVertical(linesToMove, textController);

    scrollController.jumpTo(newOffset);
  }

  void selectLine(TextEditingController controller) {
    if (controller.text.isEmpty) return;

    final start = controller.selection.baseOffset <= 0 ? -1 : controller.text.lastIndexOf('\n', controller.selection.baseOffset - 1);
    final end = controller.text.indexOf('\n', controller.selection.baseOffset);

    controller.selection = TextSelection(
      baseOffset: start == -1 ? 0 : start + 1,
      extentOffset: end == -1 ? controller.text.length : end + 1,
    );
  }

  void moveWord(int direction, TextEditingController controller) {
    // Simple word movement implementation
    if (controller.text.isEmpty) return;

    int newOffset = controller.selection.baseOffset;
    if (direction > 0) {
      // Forward: Find next whitespace or punctuation
      
      // To prevent exceeding the text length, 
      // we will handle that case when the nextSpace position had exceeded the text length
      final nextSpaceStartIndex = controller.selection.baseOffset + 1;
      final nextSpace = nextSpaceStartIndex >= 0
          ? controller.text.length
          : controller.text.indexOf(RegExp(r'\s|\p{P}', unicode: true), nextSpaceStartIndex);
      if (nextSpace != controller.text.length) {
        newOffset = nextSpace + 1; // Basic jump
      } else {
        newOffset = controller.text.length;
      }
    } else {
      // Backward: Find previous whitespace or punctuation
      
      // To prevent going out of the 0 index, which is the beginning of the document, 
      // we will need to check if the prevSpace position falls under 0
      final prevSpaceStartIndex = controller.selection.baseOffset - 1;
      final prevSpace = prevSpaceStartIndex < 0 ? -1 : controller.text.lastIndexOf(RegExp(r'\s|\p{P}', unicode: true), prevSpaceStartIndex);
      if (prevSpace != -1) {
        newOffset = prevSpace; // Basic jump to start of word
      } else {
        newOffset = 0;
      }
    }
    moveCursor(newOffset - controller.selection.baseOffset, controller);
  }

  void deleteLine(TextEditingController controller, Function(String) onChanged) {
    _preferredColumn = -1;
    // final text = widget.controller.text;
    // final selection = widget.controller.selection;
    if (controller.text.isEmpty) return;

    final start = controller.selection.baseOffset <= 0 ? -1 : controller.text.lastIndexOf('\n', controller.selection.baseOffset - 1);
    final end = controller.text.indexOf('\n', controller.selection.baseOffset);

    final deleteStart = start == -1 ? 0 : start + 1;
    final deleteEnd = end == -1 ? controller.text.length : end + 1;

    final newText = controller.text.replaceRange(deleteStart, deleteEnd, '');
    controller.value = TextEditingValue(
      text: newText,
      selection: TextSelection.collapsed(offset: deleteStart),
    );
    onChanged(newText);
  }

  Future<void> yankSelection(TextEditingController controller) async {
    final selection = controller.selection;
    final text = controller.text;
    if (selection.isCollapsed) return;

    final selectedText = text.substring(selection.start, selection.end);
    await Clipboard.setData(ClipboardData(text: selectedText));

    if (mode == EditorMode.visual || mode == EditorMode.visualLine) {
      switchMode(EditorMode.normal, controller);
      controller.selection = TextSelection.collapsed(offset: selection.baseOffset);
    }
  }

  Future<void> yankLine(TextEditingController controller) async {
    final text = controller.text;
    final selection = controller.selection;
    if (text.isEmpty) return;

    final start = selection.baseOffset <= 0 ? -1 : text.lastIndexOf('\n', selection.baseOffset - 1);
    final end = text.indexOf('\n', selection.baseOffset);

    final yankStart = start == -1 ? 0 : start + 1;
    final yankEnd = end == -1 ? text.length : end + 1;

    final lineText = text.substring(yankStart, yankEnd);

    String textToCopy = lineText;
    // Check if there is a newline at the end
    if (yankEnd <= text.length && end != -1 && text.substring(yankEnd - 1, yankEnd) == '\n') {
      if (end != -1) {
        textToCopy = text.substring(yankStart, end + 1);
      }
    } else if (end != -1) {
      // If we are not at EOF, append newline
      textToCopy = text.substring(yankStart, end + 1);
    }

    await Clipboard.setData(ClipboardData(text: textToCopy));

    // Flash or feedback could go here
    if (mode == EditorMode.visual || mode == EditorMode.visualLine) {
      switchMode(EditorMode.normal, controller);
      controller.selection = TextSelection.collapsed(offset: selection.baseOffset);
    }
  }

  void moveCursor(int delta, TextEditingController controller, {bool resetPreferredColumn = true}) {
    if (resetPreferredColumn) {
      _preferredColumn = -1;
    }

    final text = controller.text;
    var newOffset = controller.selection.baseOffset + delta;

    if (newOffset < 0) newOffset = 0;
    if (newOffset > text.length) newOffset = text.length;

    if (mode == EditorMode.visual && _visualStart != null) {
      // Extend selection
      controller.selection = TextSelection(baseOffset: _visualStart!, extentOffset: newOffset);
    } else {
      // Move caret
      controller.selection = TextSelection.collapsed(offset: newOffset);
    }
  }

  /// delta > 0 to move down, vice versa
  void moveVertical(int count, TextEditingController controller) {
    if (count == 0) return;
    if (controller.text.isEmpty) return;

    if (_preferredColumn == -1) {
      _preferredColumn = _calculateCurrentColumn(controller);
    }

    int targetStart = _getLineStart(controller, controller.selection.baseOffset);

    if (count > 0) {
      // Down
      for (int i = 0; i < count; i++) {
        final nextNL = controller.text.indexOf('\n', targetStart);
        if (nextNL == -1) {
          // Can't move down anymore
          break;
        }
        targetStart = nextNL + 1;
      }
    } else {
      // Up
      for (int i = 0; i < -count; i++) {
        if (targetStart == 0) break;
        // targetStart is start of current line.
        // targetStart-1 is the newline of previous line.
        final searchLimit = targetStart - 2;
        if (searchLimit < 0) {
          targetStart = 0;
        } else {
          final prevLineStartNL = controller.text.lastIndexOf('\n', searchLimit);
          targetStart = prevLineStartNL == -1 ? 0 : prevLineStartNL + 1;
        }
      }
    }

    // Determine target line end
    int targetEnd = controller.text.indexOf('\n', targetStart);
    if (targetEnd == -1) targetEnd = controller.text.length;

    final lineLen = targetEnd - targetStart;
    int col = _preferredColumn;
    if (col > lineLen) col = lineLen;

    final newOffset = targetStart + col;
    moveCursor(newOffset - controller.selection.baseOffset, controller, resetPreferredColumn: false);
  }

  void deleteText(int direction, TextEditingController controller, Function(String) onChanged) {
    _preferredColumn = -1;
    final selection = controller.selection;
    final text = controller.text;

    if (!selection.isCollapsed) {
      // Delete selection
      final newText = text.replaceRange(selection.start, selection.end, '');
      controller.value = TextEditingValue(
        text: newText,
        selection: TextSelection.collapsed(offset: selection.start),
      );
      onChanged(newText);
      return;
    }

    // Delete char
    if (direction < 0) {
      // Backspace
      if (selection.baseOffset > 0) {
        final newText = text.replaceRange(selection.baseOffset - 1, selection.baseOffset, '');
        controller.value = TextEditingValue(
          text: newText,
          selection: TextSelection.collapsed(offset: selection.baseOffset - 1),
        );
        onChanged(newText);
      }
    } else {
      // Delete
      if (selection.baseOffset < text.length) {
        final newText = text.replaceRange(selection.baseOffset, selection.baseOffset + 1, '');
        controller.value = TextEditingValue(
          text: newText,
          selection: TextSelection.collapsed(offset: selection.baseOffset),
        );
        onChanged(newText);
      }
    }
  }
}
