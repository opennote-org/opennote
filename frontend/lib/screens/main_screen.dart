import 'package:flutter/material.dart';
import 'package:notes/actions/handlers.dart';
import 'package:notes/inputs/global_key_handler.dart';
import 'package:notes/state/app_state_scope.dart';
import 'package:notes/state/tabs.dart';
import 'package:notes/widgets/content_area.dart';
import 'package:notes/widgets/notification_center.dart';
import 'package:notes/widgets/sidebar.dart';
import 'package:notes/services/key_mapping.dart';

class MainScreen extends StatefulWidget {
  const MainScreen({super.key});

  @override
  State<MainScreen> createState() => _MainScreenState();
}

class _MainScreenState extends State<MainScreen> {
  final GlobalKey<ScaffoldState> _scaffoldKey = GlobalKey<ScaffoldState>();
  bool _isLoading = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _handleAction(AppAction.refresh);
    });
  }

  Future<void> _handleAction(AppAction action) async {
    if (action == AppAction.saveDocument || action == AppAction.refresh) {
      setState(() => _isLoading = true);
    }

    await performAction(context, action, scaffoldKey: _scaffoldKey);

    if (mounted &&
        (action == AppAction.saveDocument || action == AppAction.refresh)) {
      setState(() => _isLoading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final appState = AppStateScope.of(context);
    final activeItem = appState.activeObject;
    final isDocumentActive = activeItem.type == ActiveObjectType.document;

    return GlobalKeyHandler(
      onAction: _handleAction,
      child: Scaffold(
        key: _scaffoldKey,
        appBar: AppBar(
          title: const Text('Notes'),
          actions: [
            const NotificationCenterButton(),
            if (isDocumentActive)
              IconButton(
                icon: const Icon(Icons.save),
                tooltip: 'Save',
                onPressed: _isLoading
                    ? null
                    : () => _handleAction(AppAction.saveDocument),
              ),

            if (activeItem.type != ActiveObjectType.none ||
                appState.username != null)
              IconButton(
                icon: const Icon(Icons.search),
                onPressed: () => _handleAction(AppAction.openSearch),
              ),
          ],
        ),
        drawer: const Drawer(child: Sidebar()),
        body: _isLoading
            ? const Center(child: CircularProgressIndicator())
            : const ContentArea(),
      ),
    );
  }
}
