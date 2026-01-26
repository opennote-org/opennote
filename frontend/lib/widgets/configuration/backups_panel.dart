import 'dart:async';

import 'package:flutter/material.dart';
import 'package:notes/state/app_state_scope.dart';

class BackupSettings extends StatefulWidget {
  const BackupSettings();

  @override
  State<BackupSettings> createState() => _BackupSettingsState();
}

class _BackupSettingsState extends State<BackupSettings> {
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