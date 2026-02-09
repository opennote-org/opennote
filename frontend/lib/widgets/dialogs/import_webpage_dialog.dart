import 'package:flutter/material.dart';

class ImportWebpageResult {
  final String text;
  final bool preserveImage;

  ImportWebpageResult({required this.text, required this.preserveImage});
}

class ImportWebpageDialog extends StatefulWidget {
  const ImportWebpageDialog({super.key});

  @override
  State<ImportWebpageDialog> createState() => _ImportWebpageDialogState();
}

class _ImportWebpageDialogState extends State<ImportWebpageDialog> {
  final TextEditingController _controller = TextEditingController();
  bool _preserveImage = false;

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
          const SizedBox(height: 16),
          CheckboxListTile(
            title: const Text('Preserve Images'),
            value: _preserveImage,
            onChanged: (value) {
              setState(() {
                _preserveImage = value ?? false;
              });
            },
            controlAffinity: ListTileControlAffinity.leading,
            contentPadding: EdgeInsets.zero,
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(
            context,
            ImportWebpageResult(
              text: _controller.text,
              preserveImage: _preserveImage,
            ),
          ),
          child: const Text('Import'),
        ),
      ],
    );
  }
}
