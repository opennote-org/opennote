import 'package:flutter/material.dart';

class ImportDatabaseDialog extends StatefulWidget {
  const ImportDatabaseDialog({super.key});

  @override
  State<ImportDatabaseDialog> createState() => _ImportDatabaseDialogState();
}

class _ImportDatabaseDialogState extends State<ImportDatabaseDialog> {
  final dbTypeController = TextEditingController(text: 'mysql');
  final hostController = TextEditingController(text: 'localhost');
  final portController = TextEditingController(text: '3306');
  final userController = TextEditingController();
  final passwordController = TextEditingController();
  final dbNameController = TextEditingController();
  final tableNameController = TextEditingController();
  final queryController = TextEditingController(text: 'SELECT * FROM table');
  final columnController = TextEditingController(text: 'content_column');

  @override
  void dispose() {
    dbTypeController.dispose();
    hostController.dispose();
    portController.dispose();
    userController.dispose();
    passwordController.dispose();
    dbNameController.dispose();
    tableNameController.dispose();
    queryController.dispose();
    columnController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Import from Database'),
      content: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(
              controller: dbTypeController,
              decoration: const InputDecoration(
                labelText: 'Database Type (mysql, postgres, sqlite)',
              ),
            ),
            Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: hostController,
                    decoration: const InputDecoration(labelText: 'Host'),
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: TextField(
                    controller: portController,
                    decoration: const InputDecoration(labelText: 'Port'),
                  ),
                ),
              ],
            ),
            TextField(
              controller: userController,
              decoration: const InputDecoration(labelText: 'Username'),
            ),
            TextField(
              controller: passwordController,
              decoration: const InputDecoration(labelText: 'Password'),
              obscureText: true,
            ),
            TextField(
              controller: dbNameController,
              decoration: const InputDecoration(labelText: 'Database Name'),
            ),
            TextField(
              controller: tableNameController,
              decoration: const InputDecoration(labelText: 'Table Name'),
            ),
            TextField(
              controller: queryController,
              decoration: const InputDecoration(labelText: 'Query'),
            ),
            TextField(
              controller: columnController,
              decoration: const InputDecoration(labelText: 'Content Column'),
            ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: () {
            final artifact = {
              "database_type": dbTypeController.text,
              "host": hostController.text,
              "port": portController.text,
              "username": userController.text,
              "password": passwordController.text,
              "database_name": dbNameController.text,
              "query": queryController.text,
              "column_to_fetch": columnController.text,
              "table_name": tableNameController.text.isEmpty
                  ? null
                  : tableNameController.text,
            };
            Navigator.pop(context, artifact);
          },
          child: const Text('Import'),
        ),
      ],
    );
  }
}
