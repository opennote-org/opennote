import 'dart:async';

import 'package:flutter/material.dart';
import 'package:notes/services/backup.dart';
import 'package:notes/services/general.dart';
import 'package:notes/services/user.dart';
import 'package:notes/state/app_state_scope.dart';

class ConfigurationPopup extends StatefulWidget {
  const ConfigurationPopup({super.key});

  @override
  State<ConfigurationPopup> createState() => _ConfigurationPopupState();
}

class _ConfigurationPopupState extends State<ConfigurationPopup> {
  int _selectedIndex = 0;

  @override
  Widget build(BuildContext context) {
    return Dialog(
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
      child: Container(
        padding: const EdgeInsets.all(24),
        constraints: const BoxConstraints(maxWidth: 900, maxHeight: 600),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Side Navigation
            SizedBox(
              width: 200,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text("Configuration", style: Theme.of(context).textTheme.headlineSmall),
                  const SizedBox(height: 24),
                  ListTile(
                    leading: const Icon(Icons.search),
                    title: const Text("Search"),
                    selected: _selectedIndex == 0,
                    onTap: () => setState(() => _selectedIndex = 0),
                  ),
                  ListTile(
                    leading: const Icon(Icons.backup),
                    title: const Text("Backup"),
                    selected: _selectedIndex == 1,
                    onTap: () => setState(() => _selectedIndex = 1),
                  ),
                ],
              ),
            ),
            const VerticalDivider(width: 48),
            // Main Content
            Expanded(
              child: IndexedStack(index: _selectedIndex, children: const [_SearchSettings(), _BackupSettings()]),
            ),
          ],
        ),
      ),
    );
  }
}

class _SearchSettings extends StatefulWidget {
  const _SearchSettings();

  @override
  State<_SearchSettings> createState() => _SearchSettingsState();
}

class _SearchSettingsState extends State<_SearchSettings> {
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
      ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text("Invalid top N")));
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
        ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text("Configurations updated")));
        setState(() {
          _isLoading = false;
        });
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
    if (_isLoading) {
      return const Center(child: CircularProgressIndicator());
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 24,
      children: [
        ..._buildSettingsOptions(),
        const Spacer(),
        Row(
          mainAxisAlignment: MainAxisAlignment.end,
          children: [
            TextButton(onPressed: () => Navigator.of(context).pop(), child: const Text("Close")),
            const SizedBox(width: 8),
            FilledButton(onPressed: _isLoading ? null : _saveConfigurations, child: const Text("Save")),
          ],
        ),
      ],
    );
  }
}

class _BackupSettings extends StatefulWidget {
  const _BackupSettings();

  @override
  State<_BackupSettings> createState() => _BackupSettingsState();
}

class _BackupSettingsState extends State<_BackupSettings> {
  final BackupService _backupService = BackupService();
  final GeneralService _generalService = GeneralService();

  bool _isLoading = true;
  List<ArchieveListItem> _backups = [];

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _loadBackups();
  }

  Future<void> _loadBackups() async {
    final appState = AppStateScope.of(context);
    final username = appState.username;

    if (username == null) return;

    try {
      final backups = await _backupService.getBackupsList(appState.dio, username);
      if (mounted) {
        setState(() {
          _backups = backups;
          _isLoading = false;
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _isLoading = false;
        });
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text("Failed to load backups: $e")));
      }
    }
  }

  Future<void> _createBackup() async {
    final appState = AppStateScope.of(context);
    final username = appState.username;

    if (username == null) return;

    setState(() => _isLoading = true);

    try {
      final taskId = await _backupService.backup(appState.dio, username);
      await _pollTask(taskId, "Backup created successfully");
      await _loadBackups();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text("Failed to create backup: $e")));
        setState(() => _isLoading = false);
      }
    }
  }

  Future<void> _restoreBackup(String archieveId) async {
    final appState = AppStateScope.of(context);

    // Confirm dialog
    final confirm = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text("Restore Backup"),
        content: const Text("Are you sure you want to restore this backup? Current data will be replaced."),
        actions: [
          TextButton(onPressed: () => Navigator.of(context).pop(false), child: const Text("Cancel")),
          FilledButton(onPressed: () => Navigator.of(context).pop(true), child: const Text("Restore")),
        ],
      ),
    );

    if (confirm != true) return;

    setState(() => _isLoading = true);

    try {
      final taskId = await _backupService.restoreBackup(appState.dio, archieveId);
      await _pollTask(taskId, "Backup restored successfully");
      setState(() => _isLoading = false);
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text("Failed to restore backup: $e")));
        setState(() => _isLoading = false);
      }
    }
  }

  Future<void> _deleteBackup(String archieveId) async {
    final appState = AppStateScope.of(context);

    // Confirm dialog
    final confirm = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text("Delete Backup"),
        content: const Text("Are you sure you want to delete this backup? This action cannot be undone."),
        actions: [
          TextButton(onPressed: () => Navigator.of(context).pop(false), child: const Text("Cancel")),
          FilledButton(onPressed: () => Navigator.of(context).pop(true), child: const Text("Delete")),
        ],
      ),
    );

    if (confirm != true) return;

    setState(() => _isLoading = true);

    try {
      await _backupService.removeBackups(appState.dio, [archieveId]);
      await _loadBackups();
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text("Backup deleted")));
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text("Failed to delete backup: $e")));
        setState(() => _isLoading = false);
      }
    }
  }

  Future<void> _pollTask(String taskId, String successMessage) async {
    final appState = AppStateScope.of(context);

    // Poll every 1 second
    while (true) {
      await Future.delayed(const Duration(seconds: 1));
      try {
        final result = await _generalService.retrieveTaskResult(appState.dio, taskId);
        if (result.status == "Completed") {
          if (mounted) {
            ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(successMessage)));
          }
          break;
        } else if (result.status == "Failed") {
          throw Exception(result.message ?? "Task failed");
        }
      } catch (e) {
        rethrow;
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Text("Backups", style: Theme.of(context).textTheme.titleMedium),
            FilledButton.icon(onPressed: _isLoading ? null : _createBackup, icon: const Icon(Icons.add), label: const Text("Backup Now")),
          ],
        ),
        const SizedBox(height: 24),
        if (_isLoading && _backups.isEmpty)
          const Center(child: CircularProgressIndicator())
        else if (_backups.isEmpty)
          const Center(child: Text("No backups found"))
        else
          Expanded(
            child: ListView.separated(
              itemCount: _backups.length,
              separatorBuilder: (context, index) => const Divider(),
              itemBuilder: (context, index) {
                final backup = _backups[index];
                return ListTile(
                  title: Text(backup.createdAt),
                  subtitle: Text("ID: ${backup.id}"),
                  trailing: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      IconButton(
                        icon: const Icon(Icons.restore),
                        tooltip: "Restore",
                        onPressed: _isLoading ? null : () => _restoreBackup(backup.id),
                      ),
                      IconButton(
                        icon: const Icon(Icons.delete),
                        tooltip: "Delete",
                        onPressed: _isLoading ? null : () => _deleteBackup(backup.id),
                      ),
                    ],
                  ),
                );
              },
            ),
          ),
      ],
    );
  }
}
