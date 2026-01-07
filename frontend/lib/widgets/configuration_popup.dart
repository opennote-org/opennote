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
  final TextEditingController _topNController = TextEditingController();
  late SupportedSearchMethod _defaultSearchMethodController;
  final UserManagementService _userService = UserManagementService();
  int? _initialChunkSize;

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
        ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text("User not logged in")));
      }
      return;
    }

    try {
      final config = await _userService.getUserConfigurations(appState.dio, username);
      if (mounted) {
        setState(() {
          _chunkSizeController.text = config.search.documentChunkSize.toString();
          _initialChunkSize = config.search.documentChunkSize;
          _defaultSearchMethodController = config.search.defaultSearchMethod;
          _topNController.text = config.search.topN.toString();
          _isLoading = false;
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _isLoading = false;
        });
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text("Failed to load configs: $e")));
      }
    }
  }

  Future<void> _saveConfigurations() async {
    final appState = AppStateScope.of(context);
    final username = appState.username;

    if (username == null) return;

    final int? chunkSize = int.tryParse(_chunkSizeController.text);
    if (chunkSize == null) {
      ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text("Invalid chunk size")));
      return;
    }

    final int? topN = int.tryParse(_topNController.text);
    if (topN == null) {
      ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text("Invalid chunk size")));
      return;
    }

    setState(() {
      _isLoading = true;
    });

    final newConfig = UserConfigurations(
      search: SearchConfiguration(documentChunkSize: chunkSize, defaultSearchMethod: _defaultSearchMethodController, topN: topN),
    );

    try {
      await _userService.updateUserConfigurations(appState.dio, username, newConfig);

      if (_initialChunkSize != null && chunkSize != _initialChunkSize) {
        await appState.reindexDocuments();
      }

      if (mounted) {
        Navigator.of(context).pop();
        ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text("Configurations updated")));
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _isLoading = false;
        });
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text("Failed to update configs: $e")));
      }
    }
  }

  /// TODO: Programatically build these options
  List<Widget> _buildSettingsOptions() {
    return [
      Text("Search Settings", style: Theme.of(context).textTheme.titleMedium),
      TextField(
        controller: _topNController,
        decoration: const InputDecoration(
          labelText: "Top N",
          border: OutlineInputBorder(),
          helperText: "How many search results to get after typing in a search query",
          helperMaxLines: 1000,
        ),
        keyboardType: TextInputType.number,
      ),
      TextField(
        controller: _chunkSizeController,
        decoration: const InputDecoration(
          labelText: "Document Maximum Chunk Size",
          border: OutlineInputBorder(),
          helperText: "Maximum size of chunks for search indexing. Adjust this if the value is beyond the model context limit",
          helperMaxLines: 1000,
        ),
        keyboardType: TextInputType.number,
      ),
      DropdownMenu<SupportedSearchMethod>(
        label: Text("Default Search Method"),
        initialSelection: _defaultSearchMethodController,
        onSelected: (selection) {
          if (selection != null) {
            setState(() {
              _defaultSearchMethodController = selection;
            });
          }
        },
        helperText: "The default way of searching",
        dropdownMenuEntries: [
          DropdownMenuEntry(value: SupportedSearchMethod.semantic, label: "Semantic"),
          DropdownMenuEntry(value: SupportedSearchMethod.keyword, label: "Keyword"),
        ],
      ),
    ];
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
            Text("Configuration", style: Theme.of(context).textTheme.headlineSmall),
            const SizedBox(height: 24),
            if (_isLoading)
              const Center(child: CircularProgressIndicator())
            else
              Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                spacing: 32,
                children: _buildSettingsOptions(),
              ),
            const SizedBox(height: 24),
            Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                TextButton(onPressed: () => Navigator.of(context).pop(), child: const Text("Cancel")),
                const SizedBox(width: 8),
                FilledButton(onPressed: _isLoading ? null : _saveConfigurations, child: const Text("Save")),
              ],
            ),
          ],
        ),
      ),
    );
  }
}
