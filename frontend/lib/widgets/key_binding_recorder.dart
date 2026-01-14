import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:notes/services/key_mapping.dart';

class KeyBindingRecorder extends StatefulWidget {
  final String? initialKey;
  final List<String> initialModifiers;
  final List<String> initialFollowingKeys;
  final ValueChanged<Map<String, dynamic>> onChanged;

  const KeyBindingRecorder({
    super.key,
    this.initialKey,
    required this.initialModifiers,
    this.initialFollowingKeys = const [],
    required this.onChanged,
  });

  @override
  State<KeyBindingRecorder> createState() => _KeyBindingRecorderState();
}

class _KeyBindingRecorderState extends State<KeyBindingRecorder> {
  bool _isRecording = false;
  late String? _key;
  late Set<String> _modifiers;
  late List<String> _followingKeys;
  final FocusNode _focusNode = FocusNode();

  @override
  void initState() {
    super.initState();
    _key = widget.initialKey;
    _modifiers = widget.initialModifiers.toSet();
    _followingKeys = List.from(widget.initialFollowingKeys);
  }

  @override
  void dispose() {
    _focusNode.dispose();
    super.dispose();
  }

  void _startRecording() {
    setState(() {
      _isRecording = true;
      _key = null;
      _modifiers.clear();
      _followingKeys.clear();
    });
    _focusNode.requestFocus();
  }

  void _stopRecording() {
    setState(() => _isRecording = false);
    // Notify change
    if (_key != null) {
      widget.onChanged({
        'key': _key,
        'modifiers': _modifiers.toList(),
        'following_keys': _followingKeys,
      });
    }
  }

  void _toggleRecording() {
    if (_isRecording) {
      _stopRecording();
    } else {
      _startRecording();
    }
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Focus(
          focusNode: _focusNode,
          onFocusChange: (hasFocus) {
            if (!hasFocus && _isRecording) {
              _stopRecording();
            }
          },
          onKeyEvent: (node, event) {
            if (!_isRecording) return KeyEventResult.ignored;
            if (event is! KeyDownEvent) return KeyEventResult.handled;

            final label = event.logicalKey.keyLabel;
            
            // Check for modifiers
            if (event.logicalKey == LogicalKeyboardKey.controlLeft || 
                event.logicalKey == LogicalKeyboardKey.controlRight) {
              if (_key == null) setState(() => _modifiers.add('ctrl'));
              return KeyEventResult.handled;
            }
            if (event.logicalKey == LogicalKeyboardKey.altLeft || 
                event.logicalKey == LogicalKeyboardKey.altRight) {
              if (_key == null) setState(() => _modifiers.add('alt'));
              return KeyEventResult.handled;
            }
            if (event.logicalKey == LogicalKeyboardKey.shiftLeft || 
                event.logicalKey == LogicalKeyboardKey.shiftRight) {
              if (_key == null) setState(() => _modifiers.add('shift'));
              return KeyEventResult.handled;
            }
            if (event.logicalKey == LogicalKeyboardKey.metaLeft || 
                event.logicalKey == LogicalKeyboardKey.metaRight) {
              if (_key == null) setState(() => _modifiers.add('meta'));
              return KeyEventResult.handled;
            }

            // If it's not a modifier, it's a key
            final keyService = KeyBindingService();
            final normalizedKey = keyService.normalizeKeyLabel(label);
            
            setState(() {
              if (_key == null) {
                _key = normalizedKey;
                // Add any held modifiers from HardwareKeyboard (in case they were held before focus)
                if (HardwareKeyboard.instance.isControlPressed) _modifiers.add('ctrl');
                if (HardwareKeyboard.instance.isAltPressed) _modifiers.add('alt');
                if (HardwareKeyboard.instance.isShiftPressed) _modifiers.add('shift');
                if (HardwareKeyboard.instance.isMetaPressed) _modifiers.add('meta');
              } else {
                // Add to following keys
                _followingKeys.add(normalizedKey);
              }
            });

            // Don't stop automatically to allow sequences
            return KeyEventResult.handled;
          },
          child: GestureDetector(
            onTap: _toggleRecording,
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
              decoration: BoxDecoration(
                border: Border.all(
                  color: _isRecording ? Theme.of(context).primaryColor : Theme.of(context).dividerColor,
                  width: _isRecording ? 2 : 1,
                ),
                borderRadius: BorderRadius.circular(4),
                color: _isRecording ? Theme.of(context).primaryColor.withOpacity(0.1) : null,
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(
                    _isRecording ? Icons.keyboard_hide : Icons.keyboard,
                    size: 20,
                    color: _isRecording ? Theme.of(context).primaryColor : null,
                  ),
                  const SizedBox(width: 8),
                  Text(
                    _isRecording 
                      ? 'Press keys...' 
                      : (_key != null ? _formatShortcut() : 'Click to record'),
                    style: TextStyle(
                      fontWeight: FontWeight.w500,
                      color: _isRecording ? Theme.of(context).primaryColor : null,
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ],
    );
  }

  String _formatShortcut() {
    final mods = _modifiers.join(' + ');
    var result = '';
    if (mods.isNotEmpty) {
      result = '$mods + $_key';
    } else {
      result = _key!;
    }
    
    if (_followingKeys.isNotEmpty) {
      result += ' ${_followingKeys.join(' ')}';
    }
    
    return result;
  }
}
