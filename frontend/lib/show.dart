import 'package:flutter/material.dart';
import 'package:notes/screens/search/search_popup.dart';
import 'package:notes/services/search.dart';
import 'package:notes/state/app_state.dart';
import 'package:notes/state/app_state_scope.dart';
import 'package:notes/widgets/configuration_popup.dart';

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
  } else if (activeItem.type == ActiveItemType.document && activeItem.id != null) {
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
    ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text('Unable to determine search scope.')));
  }
}

void showConfigurationPopup(BuildContext context) {
  showDialog(context: context, builder: (context) => const ConfigurationPopup());
}

Future<String?> showNameDialog(BuildContext context, String title, {String? initialValue}) {
  final controller = TextEditingController(text: initialValue);
  return showDialog<String>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text(title),
      content: TextField(
        controller: controller,
        autofocus: true,
        decoration: const InputDecoration(labelText: 'Name'),
      ),
      actions: [
        TextButton(onPressed: () => Navigator.pop(context), child: const Text('Cancel')),
        FilledButton(onPressed: () => Navigator.pop(context, controller.text), child: const Text('Confirm')),
      ],
    ),
  );
}
