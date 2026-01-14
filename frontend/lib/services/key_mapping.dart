import 'package:flutter/services.dart';
import 'package:flutter/foundation.dart';
import 'package:dio/dio.dart';
import 'package:notes/constants.dart';
import 'package:notes/services/user.dart';
import 'dart:async'; // Added
import 'package:collection/collection.dart'; // Added

enum KeyContext { global, editorNormal, editorInsert, editorVisual }

enum AppAction {
  // Global
  openConfig,
  openSearch,
  toggleSidebar,
  switchTabNext,
  switchTabPrevious,
  saveDocument,
  refresh,

  // Editor Navigation
  cursorMoveLeft,
  cursorMoveRight,
  cursorMoveUp,
  cursorMoveDown,

  // Editor Modes
  enterInsertMode,
  enterVisualMode,
  enterVisualLineMode, // Added
  exitInsertMode, // Esc
  exitVisualMode, // Esc
  // Editor Editing
  deleteLeft, // Backspace
  deleteRight, // Delete
  deleteLine, // Added
  yank, // Added
  yankLine, // Added
  yankSelection,
  deleteSelection,
  undo, // Added
  redo, // Added
  moveWordForward, // Added
  moveWordBackward, // Added
  gotoBeginningOfDocument, // Added
  gotoEndOfDocument, // Added
  scrollDownHalfPage, // Added
  scrollUpHalfPage, // Added

  unknown,
}

class KeyCombination {
  final String key;
  final Set<String> modifiers;
  final List<String> followingKeys;

  KeyCombination({
    required this.key,
    required this.modifiers,
    this.followingKeys = const [],
  });

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    return other is KeyCombination &&
        other.key.toLowerCase() == key.toLowerCase() &&
        setEquals(other.modifiers, modifiers) &&
        listEquals(other.followingKeys, followingKeys);
  }

  @override
  int get hashCode =>
      key.toLowerCase().hashCode ^
      Object.hashAllUnordered(modifiers) ^
      Object.hashAll(followingKeys);

  static KeyCombination? fromJson(Map<String, dynamic>? json) {
    if (json == null) return null;
    return KeyCombination(
      key: json['key'] as String,
      modifiers: (json['modifiers'] as List).cast<String>().toSet(),
      followingKeys: (json['following_keys'] as List?)?.cast<String>() ?? [],
    );
  }

  @override
  String toString() => '$modifiers + $key + $followingKeys';
}

class KeyBindingService {
  // Current active mappings: Context -> { KeyCombination -> Action }
  Map<KeyContext, Map<KeyCombination, AppAction>> _activeMappings = {};

  // Stored profiles to switch between
  Map<String, dynamic> _rawProfiles = {};

  String _currentProfileName = 'conventional_profile'; // Default
  bool _isVimEnabled = false;

  bool get isVimEnabled => _isVimEnabled;

  Future<void> fetchAndApplyConfigurations(
    Dio dio,
    UserManagementService userService,
    String username,
  ) async {
    try {
      final config = await userService.getUserConfigurationsMap(dio, username);

      if (config['key_mappings'] != null) {
        final keyMappings = config['key_mappings'];
        _rawProfiles = keyMappings;

        // Check is_vim_key_mapping_enabled
        if (keyMappings['is_vim_key_mapping_enabled'] == true) {
          _isVimEnabled = true;
          _currentProfileName = 'vim_profile';
        } else {
          _isVimEnabled = false;
          _currentProfileName = 'conventional_profile';
        }

        _applyProfile(_currentProfileName);
      }
    } catch (e) {
      debugPrint('Failed to fetch key mappings: $e');
    }
  }

  void switchProfile(String profileName) {
    if (_rawProfiles.containsKey(profileName)) {
      _currentProfileName = profileName;
      _applyProfile(profileName);
    }
  }

  void _applyProfile(String profileName) {
    final profileData = _rawProfiles[profileName];
    if (profileData == null) return;

    _activeMappings = {
      KeyContext.global: _parseSection(profileData['global'], _globalActionMap),
      KeyContext.editorNormal: _parseSection(
        profileData['editor_normal'],
        _editorActionMap,
      ),
      KeyContext.editorInsert: _parseSection(
        profileData['editor_insert'],
        _editorActionMap,
      ),
      KeyContext.editorVisual: _parseSection(
        profileData['editor_visual'],
        _editorActionMap,
      ),
    };
  }

  Map<KeyCombination, AppAction> _parseSection(
    Map<String, dynamic>? sectionData,
    Map<String, AppAction> actionNameMap,
  ) {
    final Map<KeyCombination, AppAction> result = {};
    if (sectionData == null) return result;

    sectionData.forEach((actionName, keyData) {
      if (keyData != null) {
        final combo = KeyCombination.fromJson(keyData as Map<String, dynamic>);
        final action = actionNameMap[actionName];
        if (combo != null && action != null) {
          result[combo] = action;
        }
      }
    });
    return result;
  }

  final Map<KeyContext, List<String>> _pendingKeysByContext = {};
  final Map<KeyContext, Timer> _sequenceTimersByContext = {};

  AppAction? resolve(KeyContext context, KeyEvent event) {
    // We need to construct a KeyCombination from the event
    if (event is! KeyDownEvent && event is! KeyRepeatEvent) return null;

    final keyLabel = _normalizeKeyLabel(event.logicalKey.keyLabel);
    final modifiers = <String>{};

    if (HardwareKeyboard.instance.isMetaPressed) modifiers.add('meta');
    if (HardwareKeyboard.instance.isControlPressed) modifiers.add('ctrl');
    if (HardwareKeyboard.instance.isAltPressed) modifiers.add('alt');
    if (HardwareKeyboard.instance.isShiftPressed) modifiers.add('shift');

    // Special case: If the key itself IS a modifier (e.g. user just pressed Shift), ignore
    if (['meta', 'ctrl', 'alt', 'shift'].contains(keyLabel.toLowerCase())) {
      return null;
    }

    // 1. Resolve against specific context
    final contextMappings = _activeMappings[context];
    if (contextMappings != null) {
      return _resolveSequence(context, contextMappings, keyLabel, modifiers);
    }

    return null;
  }

  AppAction? _resolveSequence(
    KeyContext context,
    Map<KeyCombination, AppAction> map,
    String keyLabel,
    Set<String> modifiers,
  ) {
    final pendingKeys = _pendingKeysByContext.putIfAbsent(
      context,
      () => <String>[],
    );

    void clearPending() {
      pendingKeys.clear();
      final timer = _sequenceTimersByContext.remove(context);
      timer?.cancel();
    }

    void restartTimer() {
      _sequenceTimersByContext[context]?.cancel();
      _sequenceTimersByContext[context] = Timer(
        const Duration(milliseconds: 1000),
        clearPending,
      );
    }

    if (pendingKeys.isEmpty) {
      // START of a sequence?
      // Find candidates that match the root key & modifiers
      final candidates = map.entries
          .where(
            (e) =>
                e.key.key.toLowerCase() == keyLabel.toLowerCase() &&
                setEquals(e.key.modifiers, modifiers),
          )
          .toList();

      if (candidates.isEmpty) return null;

      // Check if any candidate requires following keys
      final exactMatch = candidates.firstWhereOrNull(
        (e) => e.key.followingKeys.isEmpty,
      );
      final sequenceCandidates = candidates
          .where((e) => e.key.followingKeys.isNotEmpty)
          .toList();

      if (sequenceCandidates.isNotEmpty) {
        // Start buffering
        pendingKeys.add(keyLabel);
        restartTimer();

        return null; // Consumed, waiting for more
      } else if (exactMatch != null) {
        return exactMatch.value;
      }
    } else {
      // CONTINUE a sequence
      pendingKeys.add(keyLabel);
      restartTimer();

      for (final entry in map.entries) {
        final combo = entry.key;

        // 1. Root key must match first pending key
        if (combo.key.toLowerCase() != pendingKeys.first.toLowerCase()) {
          continue;
        }
      }

      // Let's implement the `_pendingCandidates` approach in next step for robustness.
      // For now, let's just try to match the sequence of labels.

      final match = map.entries.firstWhereOrNull((e) {
        final combo = e.key;
        if (combo.followingKeys.length + 1 != pendingKeys.length) return false;

        // Check root key
        if (combo.key.toLowerCase() != pendingKeys.first.toLowerCase()) {
          return false;
        }

        // Check following keys
        for (int i = 0; i < combo.followingKeys.length; i++) {
          if (combo.followingKeys[i].toLowerCase() !=
              pendingKeys[i + 1].toLowerCase()) {
            return false;
          }
        }

        return true;
      });

      if (match != null) {
        clearPending();
        return match.value;
      }

      // Check if we are still a prefix of SOMETHING
      final isPrefix = map.keys.any((combo) {
        if (pendingKeys.length > combo.followingKeys.length + 1) return false;

        if (combo.key.toLowerCase() != pendingKeys.first.toLowerCase()) {
          return false;
        }

        for (int i = 1; i < pendingKeys.length; i++) {
          if (combo.followingKeys[i - 1].toLowerCase() !=
              pendingKeys[i].toLowerCase()) {
            return false;
          }
        }
        return true;
      });

      if (!isPrefix) {
        clearPending();
      }
    }

    return null;
  }

  String normalizeKeyLabel(String label) {
    return _normalizeKeyLabel(label);
  }

  String _normalizeKeyLabel(String label) {
    // Map Flutter labels to Backend labels
    if (label == 'Arrow Left') return 'ArrowLeft';
    if (label == 'Arrow Right') return 'ArrowRight';
    if (label == 'Arrow Up') return 'ArrowUp';
    if (label == 'Arrow Down') return 'ArrowDown';
    if (label == ' ') return 'Space';
    return label;
  }

  KeyCombination? getShortcutForAction(KeyContext context, AppAction action) {
    final mapping = _activeMappings[context];
    if (mapping == null) return null;

    for (final entry in mapping.entries) {
      if (entry.value == action) {
        return entry.key;
      }
    }
    return null;
  }

  // Mappings from JSON property names to Enums
  static final Map<String, AppAction> _globalActionMap = {
    'open_config': AppAction.openConfig,
    'open_search': AppAction.openSearch,
    'toggle_sidebar': AppAction.toggleSidebar,
    'switch_tab_next': AppAction.switchTabNext,
    'switch_tab_previous': AppAction.switchTabPrevious,
    'refresh': AppAction.refresh,
  };

  static final Map<String, AppAction> _editorActionMap = {
    'cursor_move_left': AppAction.cursorMoveLeft,
    'cursor_move_right': AppAction.cursorMoveRight,
    'cursor_move_up': AppAction.cursorMoveUp,
    'cursor_move_down': AppAction.cursorMoveDown,
    'enter_insert_mode': AppAction.enterInsertMode,
    'enter_visual_mode': AppAction.enterVisualMode,
    'enter_visual_line_mode': AppAction.enterVisualLineMode,
    'exit_insert_mode': AppAction.exitInsertMode,
    'exit_visual_mode': AppAction.exitVisualMode,
    'yank': AppAction.yank,
    'yank_line': AppAction.yankLine,
    'yank_selection': AppAction.yankSelection,
    'delete_selection': AppAction.deleteSelection,
    'delete_left': AppAction.deleteLeft,
    'delete_right': AppAction.deleteRight,
    'delete_line': AppAction.deleteLine,
    'undo': AppAction.undo,
    'redo': AppAction.redo,
    'move_word_forward': AppAction.moveWordForward,
    'move_word_backward': AppAction.moveWordBackward,
    'save_document': AppAction.saveDocument,
    'goto_beginning_of_document': AppAction.gotoBeginningOfDocument,
    'goto_end_of_document': AppAction.gotoEndOfDocument,
    'scroll_down_half_page': AppAction.scrollDownHalfPage,
    'scroll_up_half_page': AppAction.scrollUpHalfPage,
  };
}
