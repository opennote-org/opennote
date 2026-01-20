import 'package:flutter/material.dart';
import 'package:notes/actions/popups.dart';
import 'package:notes/state/app_state.dart';
import 'package:notes/state/app_state_scope.dart';
import 'package:notes/services/key_mapping.dart';

typedef ActionHandler =
    Future<void> Function(
      BuildContext context,
      AppState appState,
      GlobalKey<ScaffoldState>? scaffoldKey,
    );

final Map<AppAction, ActionHandler> _actionHandlers = {
  AppAction.openConfig: (context, _, __) async =>
      showConfigurationPopup(context),
  AppAction.openSearch: (context, _, __) async => showSearchPopup(context),
  AppAction.toggleSidebar: (context, _, scaffoldKey) async {
    if (scaffoldKey?.currentState != null) {
      if (scaffoldKey!.currentState!.isDrawerOpen) {
        scaffoldKey.currentState!.closeDrawer();
      } else {
        scaffoldKey.currentState!.openDrawer();
      }
    }
  },
  AppAction.switchTabNext: (context, appState, _) async =>
      appState.switchDocumentTab(1),
  AppAction.switchTabPrevious: (context, appState, _) async =>
      appState.switchDocumentTab(-1),
  AppAction.saveDocument: (context, appState, _) async {
    try {
      await appState.saveActiveDocument();
      if (context.mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(const SnackBar(content: Text('Document saved')));
      }
    } catch (e) {
      if (context.mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text('Failed to save document: $e')));
      }
    }
  },
  AppAction.refresh: (context, appState, _) async {
    try {
      await appState.refreshAll();
      if (context.mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(const SnackBar(content: Text('Refreshed all data')));
      }
    } catch (e) {
      if (context.mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text('Failed to refresh: $e')));
      }
    }
  },
  AppAction.closeTab: (context, appState, _) async {
    final currentId = appState.activeObject.id;
    if (currentId != null) {
      appState.closeDocument(currentId);
    }
  },
};

Future<void> performAction(
  BuildContext context,
  AppAction action, {
  GlobalKey<ScaffoldState>? scaffoldKey,
}) async {
  final appState = AppStateScope.of(context);
  final handler = _actionHandlers[action];
  if (handler != null) {
    await handler(context, appState, scaffoldKey);
  }
}
