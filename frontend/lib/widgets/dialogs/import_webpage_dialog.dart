import 'package:flutter/material.dart';

class ImportWebpageDialog extends StatefulWidget {
  const ImportWebpageDialog({super.key});

  @override
  State<ImportWebpageDialog> createState() => _ImportWebpageDialogState();
}

class _ImportWebpageDialogState extends State<ImportWebpageDialog> {
  final TextEditingController _controller = TextEditingController();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Import Webpages'),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const Text('Enter URLs to import (one per line):'),
          const SizedBox(height: 8),
          TextField(
            controller: _controller,
            decoration: const InputDecoration(
              border: OutlineInputBorder(),
              hintText: 'https://example.com\nhttps://example.org',
            ),
            maxLines: 5,
            autofocus: true,
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(context, _controller.text),
          child: const Text('Import'),
        ),
      ],
    );
  }
}
