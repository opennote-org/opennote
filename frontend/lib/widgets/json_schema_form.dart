import 'package:flutter/material.dart';

class JsonSchemaForm extends StatefulWidget {
  final Map<String, dynamic> schema;
  final Map<String, dynamic> sectionSchema;
  final Map<String, dynamic> data;
  final ValueChanged<Map<String, dynamic>> onChanged;

  const JsonSchemaForm({
    super.key,
    required this.schema,
    required this.sectionSchema,
    required this.data,
    required this.onChanged,
  });

  @override
  State<JsonSchemaForm> createState() => _JsonSchemaFormState();
}

class _JsonSchemaFormState extends State<JsonSchemaForm> {
  final Map<String, TextEditingController> _controllers = {};

  @override
  void initState() {
    super.initState();
    _syncControllers();
  }

  @override
  void didUpdateWidget(covariant JsonSchemaForm oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.data != oldWidget.data) {
      _syncControllers();
    }
  }

  @override
  void dispose() {
    for (var controller in _controllers.values) {
      controller.dispose();
    }
    super.dispose();
  }

  void _syncControllers() {
    final properties = _getProperties(widget.sectionSchema);
    if (properties == null) return;

    for (var entry in properties.entries) {
      final key = entry.key;
      final value = widget.data[key];
      final fieldSchema = _resolveSchema(entry.value as Map<String, dynamic>);
      final type = fieldSchema['type'];
      
      if (type == 'integer' || type == 'number' || (type == 'string' && !fieldSchema.containsKey('enum'))) {
        if (!_controllers.containsKey(key)) {
          _controllers[key] = TextEditingController();
        }
        final text = value?.toString() ?? '';
        if (_controllers[key]!.text != text) {
          _controllers[key]!.text = text;
        }
      }
    }
  }

  Map<String, dynamic> _resolveRef(String ref) {
    final parts = ref.split('/');
    if (parts.isEmpty || parts[0] != '#') return {};
    dynamic current = widget.schema;
    for (int i = 1; i < parts.length; i++) {
      current = current[parts[i]];
      if (current == null) return {};
    }
    return current as Map<String, dynamic>;
  }

  Map<String, dynamic> _resolveSchema(Map<String, dynamic> schema) {
    if (schema.containsKey('\$ref')) {
      return _resolveRef(schema['\$ref'] as String);
    }
    return schema;
  }

  Map<String, dynamic>? _getProperties(Map<String, dynamic> schema) {
    final resolved = _resolveSchema(schema);
    return resolved['properties'] as Map<String, dynamic>?;
  }

  void _onFieldChanged(String key, dynamic value) {
    final newData = Map<String, dynamic>.from(widget.data);
    newData[key] = value;
    widget.onChanged(newData);
  }

  Widget _buildField(String key, Map<String, dynamic> schema) {
    final fieldSchema = _resolveSchema(schema);
    final type = fieldSchema['type'];
    final title = fieldSchema['title'] ?? key;
    final displayTitle = title
        .replaceAll('_', ' ')
        .split(' ')
        .map((str) => str.isNotEmpty ? '${str[0].toUpperCase()}${str.substring(1)}' : '')
        .join(' ');

    final description = fieldSchema['description'] as String?; // description might be null
    // helperText in InputDecoration expects String?

    if (fieldSchema.containsKey('enum')) {
      return DropdownButtonFormField<dynamic>(
        value: widget.data[key] ?? fieldSchema['default'],
        decoration: InputDecoration(
          labelText: displayTitle,
          helperText: description,
          border: const OutlineInputBorder(),
        ),
        items: (fieldSchema['enum'] as List)
            .map((e) => DropdownMenuItem(value: e, child: Text(e.toString())))
            .toList(),
        onChanged: (val) => _onFieldChanged(key, val),
      );
    }

    if (type == 'object') {
      return Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(displayTitle, style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          JsonSchemaForm(
            schema: widget.schema,
            sectionSchema: fieldSchema,
            data: (widget.data[key] as Map<String, dynamic>?) ?? {},
            onChanged: (val) => _onFieldChanged(key, val),
          ),
        ],
      );
    }

    if (type == 'integer' || type == 'number') {
      return TextField(
        controller: _controllers[key],
        decoration: InputDecoration(
          labelText: displayTitle,
          helperText: description,
          border: const OutlineInputBorder(),
        ),
        keyboardType: TextInputType.number,
        onChanged: (val) {
          final numVal = num.tryParse(val);
          if (numVal != null) _onFieldChanged(key, numVal);
        },
      );
    }

    if (type == 'string') {
      return TextField(
        controller: _controllers[key],
        decoration: InputDecoration(
          labelText: displayTitle,
          helperText: description,
          border: const OutlineInputBorder(),
        ),
        onChanged: (val) => _onFieldChanged(key, val),
      );
    }

    if (type == 'boolean') {
      return SwitchListTile(
        title: Text(displayTitle),
        subtitle: description != null ? Text(description) : null,
        value: widget.data[key] == true,
        onChanged: (val) => _onFieldChanged(key, val),
      );
    }

    return Text("Unknown type: $type");
  }

  @override
  Widget build(BuildContext context) {
    final properties = _getProperties(widget.sectionSchema);
    if (properties == null) return const Center(child: Text("No properties found"));

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: properties.entries.map((e) => Padding(
        padding: const EdgeInsets.only(bottom: 24),
        child: _buildField(e.key, e.value as Map<String, dynamic>),
      )).toList(),
    );
  }
}
