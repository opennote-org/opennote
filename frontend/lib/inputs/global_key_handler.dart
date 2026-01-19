import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:notes/state/app_state_scope.dart';
import 'package:notes/services/key_mapping.dart';

class GlobalKeyHandler extends StatelessWidget {
  final Widget child;
  final void Function(AppAction) onAction;

  const GlobalKeyHandler({super.key, required this.child, required this.onAction});

  @override
  Widget build(BuildContext context) {
    return Focus(
      autofocus: true,
      onKeyEvent: (node, event) {
        if (event is! KeyDownEvent) return KeyEventResult.ignored;

        final appState = AppStateScope.of(context);
        final (action, keyContext) = appState.keyBindings.resolve(event);

        if (action != null && keyContext == KeyContext.global) {
          onAction(action);
          return KeyEventResult.handled;
        }

        return KeyEventResult.ignored;
      },
      child: child,
    );
  }
}
