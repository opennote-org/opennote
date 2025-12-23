import 'package:flutter/material.dart';
import 'package:notes/screens/search/search_popup.dart';
import 'package:notes/services/search.dart';
import 'package:notes/state/app_state.dart';
import 'package:notes/state/app_state_scope.dart';
import 'package:notes/widgets/content_area.dart';
import 'package:notes/widgets/notification_center.dart';
import 'package:notes/widgets/sidebar.dart';

class MainScreen extends StatefulWidget {
  const MainScreen({super.key});

  @override
  State<MainScreen> createState() => _MainScreenState();
}

class _MainScreenState extends State<MainScreen> {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      AppStateScope.of(context).refreshCollections();
    });
  }

  // --- Import Logic ---
  bool _isLoading = false;


  Future<void> _saveActiveDocument() async {
    final appState = AppStateScope.of(context);
    setState(
      () => _isLoading = true,
    ); // Using _isLoading for general busy state
    try {
      await appState.saveActiveDocument();
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(const SnackBar(content: Text('Document saved')));
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text('Failed to save document: $e')));
      }
    } finally {
      if (mounted) setState(() => _isLoading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final appState = AppStateScope.of(context);
    final activeItem = appState.activeItem;
    final isDocumentActive = activeItem.type == ActiveItemType.document;

    return Scaffold(
      appBar: AppBar(
        title: const Text('Notes'),
        actions: [
          const NotificationCenterButton(),
          if (isDocumentActive)
            IconButton(
              icon: const Icon(Icons.save),
              tooltip: 'Save',
              onPressed: _isLoading ? null : _saveActiveDocument,
            ),

          if (activeItem.type != ActiveItemType.none ||
              appState.username != null)
            IconButton(
              icon: const Icon(Icons.search),
              onPressed: () {
                String? scopeId = activeItem.id;
                SearchScope scope = SearchScope.userspace;

                if (activeItem.type == ActiveItemType.collection &&
                    activeItem.id != null) {
                  scope = SearchScope.collection;
                  scopeId = activeItem.id;
                } else if (activeItem.type == ActiveItemType.document &&
                    activeItem.id != null) {
                  scope = SearchScope.document;
                  // If document is not in documentById (e.g. only in tree cache), try to find it
                  if (appState.documentById.containsKey(activeItem.id)) {
                    scopeId = appState.documentById[activeItem.id]?.metadataId;
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
                    builder: (context) =>
                        SearchPopup(scope: scope, scopeId: scopeId!),
                  );
                } else {
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(
                      content: Text('Unable to determine search scope.'),
                    ),
                  );
                }
              },
            ),
        ],
      ),
      drawer: const Drawer(child: Sidebar()),
      body: _isLoading
          ? const Center(child: CircularProgressIndicator())
          : const ContentArea(),
    );
  }
}
