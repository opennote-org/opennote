import 'package:flutter/material.dart';
import 'package:notes/screens/search/search_popup.dart';
import 'package:notes/services/search.dart';
import 'package:notes/state/app_state.dart';
import 'package:notes/state/app_state_scope.dart';
import 'package:notes/widgets/configuration_popup.dart';
import 'package:notes/services/key_mapping.dart';

typedef ActionHandler = Future<void> Function(
  BuildContext context,
  AppState appState,
  GlobalKey<ScaffoldState>? scaffoldKey,
);

final Map<AppAction, ActionHandler> _actionHandlers = {
  AppAction.openConfig:
      (context, _, __) async => showConfigurationPopup(context),
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
  AppAction.switchTabNext:
      (context, appState, _) async => _switchTab(appState, 1),
  AppAction.switchTabPrevious:
      (context, appState, _) async => _switchTab(appState, -1),
  AppAction.saveDocument: (context, appState, _) async {
    try {
      await appState.saveActiveDocument();
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Document saved')),
        );
      }
    } catch (e) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Failed to save document: $e')),
        );
      }
    }
  },
  AppAction.refresh: (context, appState, _) async {
    try {
      await appState.refreshAll();
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Refreshed all data')),
        );
      }
    } catch (e) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Failed to refresh: $e')),
        );
      }
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

void _switchTab(AppState appState, int offset) {
  if (appState.openDocumentIds.isEmpty) return;

  final currentId = appState.activeItem.id;
  if (currentId == null) {
    if (appState.openDocumentIds.isNotEmpty) {
      appState.setActiveItem(
        ActiveItemType.document,
        appState.openDocumentIds.first,
      );
    }
    return;
  }

  final currentIndex = appState.openDocumentIds.indexOf(currentId);
  if (currentIndex == -1) return;

  final newIndex =
      (currentIndex + offset + appState.openDocumentIds.length) %
      appState.openDocumentIds.length;
  appState.setActiveItem(
    ActiveItemType.document,
    appState.openDocumentIds[newIndex],
  );
}

void showSearchPopup(BuildContext context) {
  final appState = AppStateScope.of(context);
  final activeItem = appState.activeItem;

  if (activeItem.type == ActiveItemType.none && appState.username == null) {
    return;
  }

  String? scopeId = activeItem.id;
  SearchScope scope = SearchScope.userspace;

  if (activeItem.type == ActiveItemType.collection && activeItem.id != null) {
    scope = SearchScope.collection;
    scopeId = activeItem.id;
  } else if (activeItem.type == ActiveItemType.document &&
      activeItem.id != null) {
    scope = SearchScope.document;
    // If document is not in documentById (e.g. only in tree cache), try to find it
    if (appState.documentById.containsKey(activeItem.id)) {
      scopeId = appState.documentById[activeItem.id]?.id;
    } else {
      // Fallback: Use activeItem.id directly as we expect it to be the metadataId
      scopeId = activeItem.id;
    }
  } else {
    // Default to Userspace search if no specific item is active or relevant
    scope = SearchScope.userspace;
    scopeId = appState.username;
  }

  if (scopeId != null) {
    showDialog(
      context: context,
      builder: (context) => SearchPopup(scope: scope, scopeId: scopeId!),
    );
  } else {
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(content: Text('Unable to determine search scope.')),
    );
  }
}

void showConfigurationPopup(BuildContext context) {
  showDialog(
    context: context,
    builder: (context) => const ConfigurationPopup(),
  );
}

Future<String?> showNameDialog(
  BuildContext context,
  String title, {
  String? initialValue,
}) {
  final controller = TextEditingController(text: initialValue);
  return showDialog<String>(
    context: context,
    builder:
        (context) => AlertDialog(
          title: Text(title),
          content: TextField(
            controller: controller,
            autofocus: true,
            decoration: const InputDecoration(labelText: 'Name'),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed:
                  () => Navigator.pop(context, controller.text),
              child: const Text('Confirm'),
            ),
          ],
        ),
  );
}
