import 'package:flutter/material.dart';
import 'package:notes/widgets/key_binding_recorder.dart';

class JsonSchemaForm extends StatefulWidget {
  final Map<String, dynamic> schema;
  final Map<String, dynamic> sectionSchema;
  final Map<String, dynamic> data;
  final ValueChanged<Map<String, dynamic>> onChanged;

  const JsonSchemaForm({super.key, required this.schema, required this.sectionSchema, required this.data, required this.onChanged});

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
      final resolved = _resolveRef(schema['\$ref'] as String);
      // Merge resolved schema with current schema to preserve descriptions/titles
      // defined at the property level.
      return {...resolved, ...schema};
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

  Map<String, dynamic> _resolveFieldSchema(Map<String, dynamic> schema) {
    // Handle nullable types (e.g. ["string", "null"]) or anyOf/oneOf
    var fieldSchema = _resolveSchema(schema);

    // Check if type is a list (nullable)
    if (fieldSchema['type'] is List) {
      final types = fieldSchema['type'] as List;
      // Find the first non-null type
      final actualType = types.firstWhere((t) => t != 'null', orElse: () => 'string');
      // We create a new schema map forcing the single type for rendering logic
      fieldSchema = Map<String, dynamic>.from(fieldSchema);
      fieldSchema['type'] = actualType;
    }

    // Handle anyOf / oneOf (often used for optional fields)
    if (fieldSchema.containsKey('anyOf')) {
      final options = fieldSchema['anyOf'] as List;
      // Try to find the non-null option
      for (var opt in options) {
        if (opt is Map<String, dynamic>) {
          // Recursively resolve the option in case it's a ref
          final resolvedOpt = _resolveSchema(opt);
          if (resolvedOpt['type'] != 'null') {
            // Merge the resolved option into the field schema
            fieldSchema = {...fieldSchema, ...resolvedOpt, 'type': resolvedOpt['type'] ?? fieldSchema['type']};
            break;
          }
        }
      }
    }
    return fieldSchema;
  }

  String _getDisplayTitle(String key, Map<String, dynamic> fieldSchema) {
    final title = fieldSchema['title'] ?? key;
    return title.replaceAll('_', ' ').split(' ').map((str) => str.isNotEmpty ? '${str[0].toUpperCase()}${str.substring(1)}' : '').join(' ');
  }

  Widget _buildField(String key, Map<String, dynamic> schema) {
    final fieldSchema = _resolveFieldSchema(schema);
    final type = fieldSchema['type'];
    final displayTitle = _getDisplayTitle(key, fieldSchema);
    final description = fieldSchema['description'] as String?;

    if (fieldSchema.containsKey('enum')) {
      return _buildEnumField(key, fieldSchema, displayTitle, description);
    }

    if (type == 'object') {
      return _buildObjectField(key, fieldSchema, displayTitle, description);
    }

    if (type == 'integer' || type == 'number') {
      return _buildNumericField(key, displayTitle, description);
    }

    if (type == 'string') {
      return _buildStringField(key, displayTitle, description);
    }

    if (type == 'boolean') {
      return _buildBooleanField(key, displayTitle, description);
    }

    if (type == 'array') {
      return _buildArrayField(key, fieldSchema, displayTitle, description);
    }

    return Text("Unknown type: $type");
  }

  Widget _buildEnumField(String key, Map<String, dynamic> fieldSchema, String displayTitle, String? description) {
    return DropdownButtonFormField<dynamic>(
      value: widget.data[key] ?? fieldSchema['default'],
      decoration: InputDecoration(
        labelText: displayTitle,
        helperText: description,
        border: const OutlineInputBorder(),
        hintMaxLines: 1000,
        errorMaxLines: 1000,
        helperMaxLines: 1000,
      ),
      items: (fieldSchema['enum'] as List).map((e) => DropdownMenuItem(value: e, child: Text(e.toString()))).toList(),
      onChanged: (val) => _onFieldChanged(key, val),
    );
  }

  Widget _buildObjectField(String key, Map<String, dynamic> fieldSchema, String displayTitle, String? description) {
    // Check if this is a KeyCombination (has 'key' and 'modifiers')
    final properties = _getProperties(fieldSchema);
    if (properties != null && properties.containsKey('key') && properties.containsKey('modifiers')) {
      return _buildKeyBindingRecorderField(key, displayTitle, description);
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(displayTitle, style: Theme.of(context).textTheme.titleMedium, maxLines: 1000),
        if (description != null) ...[
          const SizedBox(height: 4),
          Text(description, style: Theme.of(context).textTheme.bodySmall, maxLines: 1000),
        ],
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

  Widget _buildKeyBindingRecorderField(String key, String displayTitle, String? description) {
    // Use KeyBindingRecorder
    final currentData = (widget.data[key] as Map<String, dynamic>?) ?? {};
    final currentKey = currentData['key'] as String?;
    final currentModifiers = (currentData['modifiers'] as List?)?.cast<String>() ?? [];
    final currentFollowingKeys = (currentData['following_keys'] as List?)?.cast<String>() ?? [];

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(displayTitle, style: Theme.of(context).textTheme.titleMedium),
        if (description != null) Text(description, style: Theme.of(context).textTheme.bodySmall),
        const SizedBox(height: 8),
        KeyBindingRecorder(
          initialKey: currentKey,
          initialModifiers: currentModifiers,
          initialFollowingKeys: currentFollowingKeys,
          onChanged: (newValue) => _onFieldChanged(key, newValue),
        ),
      ],
    );
  }

  Widget _buildNumericField(String key, String displayTitle, String? description) {
    return TextField(
      controller: _controllers[key],
      decoration: InputDecoration(
        labelText: displayTitle,
        helperText: description,
        border: const OutlineInputBorder(),
        hintMaxLines: 1000,
        errorMaxLines: 1000,
        helperMaxLines: 1000,
      ),
      keyboardType: TextInputType.number,
      onChanged: (val) {
        final numVal = num.tryParse(val);
        if (numVal != null) _onFieldChanged(key, numVal);
      },
    );
  }

  Widget _buildStringField(String key, String displayTitle, String? description) {
    return TextField(
      controller: _controllers[key],
      decoration: InputDecoration(
        labelText: displayTitle,
        helperText: description,
        border: const OutlineInputBorder(),
        hintMaxLines: 1000,
        errorMaxLines: 1000,
        helperMaxLines: 1000,
      ),
      onChanged: (val) => _onFieldChanged(key, val),
    );
  }

  Widget _buildBooleanField(String key, String displayTitle, String? description) {
    return SwitchListTile(
      title: Text(displayTitle),
      subtitle: description != null ? Text(description) : null,
      value: widget.data[key] == true,
      onChanged: (val) => _onFieldChanged(key, val),
    );
  }

  Widget _buildArrayField(String key, Map<String, dynamic> fieldSchema, String displayTitle, String? description) {
    final itemsSchema = _resolveSchema(fieldSchema['items'] as Map<String, dynamic>);
    if (itemsSchema.containsKey('enum')) {
      // Multi-select for enums (e.g. modifiers)
      final options = (itemsSchema['enum'] as List).cast<String>();
      final currentValues = (widget.data[key] as List?)?.cast<String>() ?? [];

      return Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(displayTitle, style: Theme.of(context).textTheme.titleMedium),
          if (description != null) Text(description, style: Theme.of(context).textTheme.bodySmall),
          Wrap(
            spacing: 8,
            children: options.map((option) {
              final isSelected = currentValues.contains(option);
              return FilterChip(
                label: Text(option),
                selected: isSelected,
                onSelected: (selected) {
                  final newValues = List<String>.from(currentValues);
                  if (selected) {
                    newValues.add(option);
                  } else {
                    newValues.remove(option);
                  }
                  _onFieldChanged(key, newValues);
                },
              );
            }).toList(),
          ),
        ],
      );
    }
    return Text("Unknown array type");
  }

  @override
  Widget build(BuildContext context) {
    final properties = _getProperties(widget.sectionSchema);
    if (properties == null) {
      return const Center(child: Text("No properties found"));
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: properties.entries
          .map((e) => Padding(padding: const EdgeInsets.only(bottom: 24), child: _buildField(e.key, e.value as Map<String, dynamic>)))
          .toList(),
    );
  }
}
