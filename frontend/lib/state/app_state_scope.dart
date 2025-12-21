import 'package:flutter/widgets.dart';
import 'package:notes/state/app_state.dart';

class AppStateScope extends InheritedNotifier<AppState> {
  const AppStateScope({super.key, required AppState notifier, required Widget child}) : super(notifier: notifier, child: child);
  static AppState of(BuildContext context) => context.dependOnInheritedWidgetOfExactType<AppStateScope>()!.notifier!;
}