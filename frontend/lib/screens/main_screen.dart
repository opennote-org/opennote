import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:notes/show.dart';
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
  void _showSearchPopup() => showSearchPopup(context);
  void _showConfigurationPopup() => showConfigurationPopup(context);

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
    setState(() => _isLoading = true); // Using _isLoading for general busy state
    try {
      await appState.saveActiveDocument();
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text('Document saved')));
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('Failed to save document: $e')));
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

    return Focus(
      autofocus: true,
      onKeyEvent: (node, event) {
        if (const SingleActivator(LogicalKeyboardKey.keyP, control: true)
            .accepts(event, HardwareKeyboard.instance)) {
          _showSearchPopup();
          return KeyEventResult.handled;
        }
        if (const SingleActivator(LogicalKeyboardKey.keyC, control: true)
            .accepts(event, HardwareKeyboard.instance)) {
          _showConfigurationPopup();
          return KeyEventResult.handled;
        }
        if (const SingleActivator(LogicalKeyboardKey.keyS, control: true)
            .accepts(event, HardwareKeyboard.instance)) {
          if (isDocumentActive) {
            _saveActiveDocument();
            return KeyEventResult.handled;
          }
        }
        return KeyEventResult.ignored;
      },
      child: Scaffold(
          appBar: AppBar(
            title: const Text('Notes'),
            actions: [
              const NotificationCenterButton(),
              if (isDocumentActive)
                IconButton(icon: const Icon(Icons.save), tooltip: 'Save', onPressed: _isLoading ? null : _saveActiveDocument),

              if (activeItem.type != ActiveItemType.none || appState.username != null)
                IconButton(icon: const Icon(Icons.search), onPressed: _showSearchPopup),
            ],
          ),
          drawer: const Drawer(child: Sidebar()),
          body: _isLoading ? const Center(child: CircularProgressIndicator()) : const ContentArea(),
        ),
    );
  }
}
