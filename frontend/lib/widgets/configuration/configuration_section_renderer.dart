import 'package:flutter/material.dart';

abstract class ConfigurationSectionRenderer {
  const ConfigurationSectionRenderer();

  bool canRender(String sectionKey, Map<String, dynamic> sectionSchema);

  Widget buildBody({
    required BuildContext context,
    required String sectionKey,
    required Map<String, dynamic> fullSchema,
    required Map<String, dynamic> sectionSchema,
    required Map<String, dynamic> sectionData,
    required ValueChanged<Map<String, dynamic>> onSectionChanged,
  });
}

