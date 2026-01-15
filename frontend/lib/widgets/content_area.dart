import 'package:flutter/material.dart';
import 'package:notes/actions.dart';
import 'package:notes/state/app_state.dart';
import 'package:notes/state/app_state_scope.dart';
import 'package:notes/services/key_mapping.dart';
import 'package:notes/widgets/document_editor.dart';

class ContentArea extends StatelessWidget {
  const ContentArea({super.key});
  
  @override
  Widget build(BuildContext context) {
    final appState = AppStateScope.of(context);
    
    // Update last document if the active item is a document
    if (appState.activeItem.type == ActiveItemType.document && appState.activeItem.id != null) {
      appState.lastActiveDocumentId = appState.activeItem.id;
    }

    if (appState.openDocumentIds.isEmpty) {
      // Resolve shortcuts for display
      final searchShortcut =
          appState.keyBindings
              .getShortcutForAction(KeyContext.global, AppAction.openSearch)
              ?.toString() ??
          'Ctrl + P';

      final configShortcut =
          appState.keyBindings
              .getShortcutForAction(KeyContext.global, AppAction.openConfig)
              ?.toString() ??
          'Cmd + ,';

      return Focus(
        autofocus: true,
        child: Builder(
          builder: (context) {
            return GestureDetector(
              behavior: HitTestBehavior.translucent,
              onTap: () =>
                  FocusScope.of(context).requestFocus(Focus.of(context)),
              child: Center(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    _buildShortcutRow(
                      showSearchPopup,
                      context,
                      'Search',
                      searchShortcut,
                      Icons.search,
                    ),
                    _buildShortcutRow(
                      showConfigurationPopup,
                      context,
                      'Configurations',
                      configShortcut,
                      Icons.settings,
                    ),
                  ],
                ),
              ),
            );
          },
        ),
      );
    }

    return Column(
      children: [
        // Tab Bar
        SizedBox(
          height: 48,
          child: ListView.builder(
            scrollDirection: Axis.horizontal,
            itemCount: appState.openDocumentIds.length,
            itemBuilder: (context, index) {
              final docId = appState.openDocumentIds[index];
              final doc = appState.documentById[docId];
              final isActive = docId == appState.activeItem.id;

              return InkWell(
                onTap: () =>
                    appState.setActiveItem(ActiveItemType.document, docId),
                child: Container(
                  margin: EdgeInsets.only(top: 5, bottom: 5),
                  width: 200, // Fixed width for tabs
                  padding: const EdgeInsets.symmetric(horizontal: 8),
                  alignment: Alignment.center,
                  decoration: BoxDecoration(
                    color: isActive
                        ? Theme.of(context).colorScheme.surface
                        : Theme.of(context).colorScheme.surfaceContainerHighest,
                    border: Border(
                      right: BorderSide(color: Theme.of(context).dividerColor),
                      top: isActive
                          ? BorderSide(
                              color: Theme.of(context).primaryColor,
                              width: 2,
                            )
                          : BorderSide.none,
                    ),
                  ),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Expanded(
                        child: Text(
                          doc?.title ?? 'Loading...',
                          overflow: TextOverflow.ellipsis,
                          maxLines: 1,
                        ),
                      ),
                      const SizedBox(width: 4),
                      InkWell(
                        onTap: () => appState.closeDocument(docId),
                        child: const Icon(Icons.close, size: 16),
                      ),
                    ],
                  ),
                ),
              );
            },
          ),
        ),
        // Content
        Expanded(
          child: appState.lastActiveDocumentId != null
              ? DocumentEditor(
                  key: ValueKey(appState.lastActiveDocumentId),
                  documentId: appState.lastActiveDocumentId!,
                )
              : const SizedBox(),
        ),
      ],
    );
  }

  Widget _buildShortcutRow(
    void Function(BuildContext context) popupFunction,
    BuildContext context,
    String label,
    String shortcut,
    IconData icon,
  ) {
    void _popupFunction() => popupFunction(context);

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          GestureDetector(
            onTap: _popupFunction,
            child: Row(
              spacing: 12,
              children: [
                Icon(
                  icon,
                  size: 20,
                  color: Theme.of(context).colorScheme.secondary,
                ),
                Text(
                  label,
                  style: Theme.of(context).textTheme.bodyLarge?.copyWith(
                    color: Theme.of(context).colorScheme.secondary,
                  ),
                ),
                Text(
                  shortcut,
                  style: Theme.of(context).textTheme.bodyLarge?.copyWith(
                    fontWeight: FontWeight.bold,
                    color: Theme.of(context).colorScheme.onSurface,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
