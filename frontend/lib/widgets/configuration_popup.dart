import 'package:flutter/material.dart';
import 'package:notes/services/user.dart';
import 'package:notes/state/app_state_scope.dart';

class ConfigurationPopup extends StatefulWidget {
  const ConfigurationPopup({super.key});

  @override
  State<ConfigurationPopup> createState() => _ConfigurationPopupState();
}

class _ConfigurationPopupState extends State<ConfigurationPopup> {
  bool _isLoading = true;
  final TextEditingController _chunkSizeController = TextEditingController();
  final UserManagementService _userService = UserManagementService();

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _loadConfigurations();
  }

  Future<void> _loadConfigurations() async {
    final appState = AppStateScope.of(context);
    final username = appState.username;

    if (username == null) {
      if (mounted) {
        setState(() {
          _isLoading = false;
        });
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text("User not logged in")),
        );
      }
      return;
    }

    try {
      final config = await _userService.getUserConfigurations(appState.dio, username);
      if (mounted) {
        setState(() {
          _chunkSizeController.text = config.search.documentChunkSize.toString();
          _isLoading = false;
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _isLoading = false;
        });
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text("Failed to load configs: $e")),
        );
      }
    }
  }

  Future<void> _saveConfigurations() async {
    final appState = AppStateScope.of(context);
    final username = appState.username;

    if (username == null) return;
    
    final int? chunkSize = int.tryParse(_chunkSizeController.text);
    if (chunkSize == null) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text("Invalid chunk size")),
      );
      return;
    }

    setState(() {
      _isLoading = true;
    });

    final newConfig = UserConfigurations(
      search: SearchConfiguration(documentChunkSize: chunkSize),
    );

    try {
      await _userService.updateUserConfigurations(appState.dio, username, newConfig);
      if (mounted) {
        Navigator.of(context).pop();
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text("Configurations updated")),
        );
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _isLoading = false;
        });
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text("Failed to update configs: $e")),
        );
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return Dialog(
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
      child: Container(
        padding: const EdgeInsets.all(24),
        constraints: const BoxConstraints(maxWidth: 400),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              "Configuration",
              style: Theme.of(context).textTheme.headlineSmall,
            ),
            const SizedBox(height: 24),
            if (_isLoading)
              const Center(child: CircularProgressIndicator())
            else
              Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text("Search Settings",
                      style: Theme.of(context).textTheme.titleMedium),
                  const SizedBox(height: 16),
                  TextField(
                    controller: _chunkSizeController,
                    decoration: const InputDecoration(
                      labelText: "Document Chunk Size",
                      border: OutlineInputBorder(),
                      helperText: "Size of chunks for search indexing",
                    ),
                    keyboardType: TextInputType.number,
                  ),
                ],
              ),
            const SizedBox(height: 24),
            Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                TextButton(
                  onPressed: () => Navigator.of(context).pop(),
                  child: const Text("Cancel"),
                ),
                const SizedBox(width: 8),
                FilledButton(
                  onPressed: _isLoading ? null : _saveConfigurations,
                  child: const Text("Save"),
                ),
              ],
            )
          ],
        ),
      ),
    );
  }
}
