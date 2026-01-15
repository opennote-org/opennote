import 'dart:async';

import 'package:flutter/material.dart';
import 'package:notes/state/app_state.dart';
import 'package:notes/state/app_state_scope.dart';
import 'package:notes/widgets/configuration/configuration_section_renderer.dart';
import 'package:notes/widgets/configuration/default_json_schema_section_renderer.dart';
import 'package:notes/widgets/configuration/key_mappings_section_renderer.dart';

class ConfigurationPopup extends StatefulWidget {
  const ConfigurationPopup({super.key});

  @override
  State<ConfigurationPopup> createState() => _ConfigurationPopupState();
}

class _ConfigurationPopupState extends State<ConfigurationPopup> {
  int _selectedIndex = 0;
  bool _isLoading = true;
  Map<String, dynamic>? _schema;
  Map<String, dynamic> _config = {};
  int? _initialChunkSize;
  final List<ConfigurationSectionRenderer> _sectionRenderers = const [
    KeyMappingsSectionRenderer(),
    DefaultJsonSchemaSectionRenderer(),
  ];

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _loadData();
  }

  Future<void> _loadData() async {
    final appState = AppStateScope.of(context);
    final username = appState.username;
    if (username == null) return;

    try {
      final schema = await appState.users.getUserConfigurationsSchemars(
        appState.dio,
      );
      final config = await appState.users.getUserConfigurationsMap(
        appState.dio,
        username,
      );

      if (mounted) {
        setState(() {
          _schema = schema;
          _config = config;
          _isLoading = false;
          if (_config.containsKey('search') &&
              _config['search'] is Map &&
              (_config['search'] as Map).containsKey('document_chunk_size')) {
            _initialChunkSize =
                (_config['search'] as Map)['document_chunk_size'] as int?;
          }
        });
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text("Error: $e")));
        setState(() => _isLoading = false);
      }
    }
  }

  Future<void> _saveData() async {
    final appState = AppStateScope.of(context);
    final username = appState.username;
    if (username == null) return;

    setState(() => _isLoading = true);

    try {
      await appState.users.updateUserConfigurationsMap(
        appState.dio,
        username,
        _config,
      );

      // Reindex check
      if (_initialChunkSize != null &&
          _config.containsKey('search') &&
          _config['search'] is Map &&
          (_config['search'] as Map).containsKey('document_chunk_size')) {
        final newChunkSize =
            (_config['search'] as Map)['document_chunk_size'] as int?;
        if (newChunkSize != null && newChunkSize != _initialChunkSize) {
          await appState.reindexDocuments();
          _initialChunkSize = newChunkSize;
        }
      }

      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(const SnackBar(content: Text("Configurations updated")));
        await appState.refreshAll();
        setState(() => _isLoading = false);
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text("Failed to update configs: $e")));
        setState(() => _isLoading = false);
      }
    }
  }

  List<String> _getSchemaProperties() {
    if (_schema == null || !_schema!.containsKey('properties')) return [];
    return (_schema!['properties'] as Map<String, dynamic>).keys.toList();
  }

  @override
  Widget build(BuildContext context) {
    final schemaProperties = _getSchemaProperties();
    // Add Backup as the last item
    final tabs = [...schemaProperties, "Backup"];

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
                  Text(
                    "Configuration",
                    style: Theme.of(context).textTheme.headlineSmall,
                  ),
                  const SizedBox(height: 24),
                  if (_isLoading && _schema == null)
                    const Center(child: CircularProgressIndicator())
                  else
                    Expanded(
                      child: ListView.builder(
                        itemCount: tabs.length,
                        itemBuilder: (context, index) {
                          final title = tabs[index];
                          // Capitalize
                          final displayTitle = title == "Backup"
                              ? title
                              : title
                                    .split('_')
                                    .map(
                                      (word) => word.isNotEmpty
                                          ? word[0].toUpperCase() +
                                                word.substring(1)
                                          : '',
                                    )
                                    .join(' ');

                          IconData icon = Icons.settings;
                          if (title == "search") icon = Icons.search;
                          if (title == "Backup") icon = Icons.backup;

                          return ListTile(
                            leading: Icon(icon),
                            title: Text(displayTitle),
                            selected: _selectedIndex == index,
                            onTap: () => setState(() => _selectedIndex = index),
                          );
                        },
                      ),
                    ),
                ],
              ),
            ),
            const VerticalDivider(width: 48),
            // Main Content
            Expanded(
              child: _isLoading && _schema == null
                  ? const Center(child: CircularProgressIndicator())
                  : _buildContent(tabs),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildContent(List<String> tabs) {
    if (_selectedIndex >= tabs.length) {
      return const SizedBox();
    }

    final selectedTab = tabs[_selectedIndex];

    if (selectedTab == "Backup") {
      return const _BackupSettings();
    }

    final sectionKey = selectedTab;
    final sectionSchema =
        (_schema!['properties'] as Map<String, dynamic>)[sectionKey];

    if (sectionSchema is! Map<String, dynamic>) {
      return const SizedBox();
    }

    final description = sectionSchema['description'] as String?;
    final sectionData = (_config[sectionKey] as Map<String, dynamic>?) ?? {};
    final renderer = _sectionRenderers.firstWhere(
      (r) => r.canRender(sectionKey, sectionSchema),
    );

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (description != null) ...[
          Padding(
            padding: const EdgeInsets.only(bottom: 16.0),
            child: Text(
              description,
              style: Theme.of(context).textTheme.bodyMedium,
            ),
          ),
          const Divider(),
          const SizedBox(height: 16),
        ],
        Expanded(
          child: renderer.buildBody(
            context: context,
            sectionKey: sectionKey,
            fullSchema: _schema!,
            sectionSchema: sectionSchema,
            sectionData: sectionData,
            onSectionChanged: (newData) {
              setState(() {
                _config[sectionKey] = newData;
              });
            },
          ),
        ),
        const SizedBox(height: 16),
        Row(
          mainAxisAlignment: MainAxisAlignment.end,
          children: [
            TextButton(
              onPressed: () => Navigator.of(context).pop(),
              child: const Text("Close"),
            ),
            const SizedBox(width: 8),
            FilledButton(
              onPressed: _isLoading ? null : _saveData,
              child: const Text("Save"),
            ),
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
  bool _isLoading = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _loadBackups();
    });
  }

  Future<void> _loadBackups() async {
    final appState = AppStateScope.of(context);
    if (appState.username == null) return;

    setState(() => _isLoading = true);
    await appState.fetchBackups();
    if (mounted) {
      setState(() => _isLoading = false);
    }
  }

  Future<void> _createBackup() async {
    final appState = AppStateScope.of(context);
    setState(() => _isLoading = true);
    try {
      await appState.createBackup();
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(const SnackBar(content: Text("Backup task started")));
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text("Failed to start backup: $e")));
      }
    } finally {
      if (mounted) setState(() => _isLoading = false);
    }
  }

  Future<void> _restoreBackup(String archieveId) async {
    final appState = AppStateScope.of(context);

    // Confirm dialog
    final confirm = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text("Restore Backup"),
        content: const Text(
          "Are you sure you want to restore this backup? Current data will be replaced.",
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text("Cancel"),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: const Text("Restore"),
          ),
        ],
      ),
    );

    if (confirm != true) return;

    setState(() => _isLoading = true);
    try {
      await appState.restoreBackup(archieveId);
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(const SnackBar(content: Text("Restore task started")));
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text("Failed to start restore: $e")));
      }
    } finally {
      if (mounted) setState(() => _isLoading = false);
    }
  }

  Future<void> _deleteBackup(String archieveId) async {
    final appState = AppStateScope.of(context);

    // Confirm dialog
    final confirm = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text("Delete Backup"),
        content: const Text(
          "Are you sure you want to delete this backup? This action cannot be undone.",
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text("Cancel"),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: const Text("Delete"),
          ),
        ],
      ),
    );

    if (confirm != true) return;

    setState(() => _isLoading = true);

    try {
      await appState.deleteBackup(archieveId);
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(const SnackBar(content: Text("Backup deleted")));
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text("Failed to delete backup: $e")));
      }
    } finally {
      if (mounted) setState(() => _isLoading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final appState = AppStateScope.of(context);
    final backups = appState.backups;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Text("Backups", style: Theme.of(context).textTheme.titleMedium),
            FilledButton.icon(
              onPressed: _isLoading ? null : _createBackup,
              icon: const Icon(Icons.add),
              label: const Text("Backup Now"),
            ),
          ],
        ),
        const SizedBox(height: 24),
        if (_isLoading && backups.isEmpty)
          const Center(child: CircularProgressIndicator())
        else if (backups.isEmpty)
          const Center(child: Text("No backups found"))
        else
          Expanded(
            child: ListView.separated(
              itemCount: backups.length,
              separatorBuilder: (context, index) => const Divider(),
              itemBuilder: (context, index) {
                final backup = backups[index];
                return ListTile(
                  title: Text(backup.createdAt),
                  subtitle: Text("ID: ${backup.id}"),
                  trailing: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      IconButton(
                        icon: const Icon(Icons.restore),
                        tooltip: "Restore",
                        onPressed: _isLoading
                            ? null
                            : () => _restoreBackup(backup.id),
                      ),
                      IconButton(
                        icon: const Icon(Icons.delete),
                        tooltip: "Delete",
                        onPressed: _isLoading
                            ? null
                            : () => _deleteBackup(backup.id),
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
