import 'package:flutter/material.dart';
import 'package:notes/widgets/configuration/configuration_section_renderer.dart';
import 'package:notes/widgets/json_schema_form.dart';

class DefaultJsonSchemaSectionRenderer extends ConfigurationSectionRenderer {
  const DefaultJsonSchemaSectionRenderer();

  @override
  bool canRender(String sectionKey, Map<String, dynamic> sectionSchema) => true;

  @override
  Widget buildBody({
    required BuildContext context,
    required String sectionKey,
    required Map<String, dynamic> fullSchema,
    required Map<String, dynamic> sectionSchema,
    required Map<String, dynamic> sectionData,
    required ValueChanged<Map<String, dynamic>> onSectionChanged,
  }) {
    return SingleChildScrollView(
      child: JsonSchemaForm(
        schema: fullSchema,
        sectionSchema: sectionSchema,
        data: sectionData,
        onChanged: onSectionChanged,
      ),
    );
  }
}

