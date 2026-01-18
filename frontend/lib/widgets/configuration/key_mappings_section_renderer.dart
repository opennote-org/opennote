import 'package:flutter/material.dart';
import 'package:notes/widgets/configuration/configuration_section_renderer.dart';
import 'package:notes/widgets/configuration/schema_resolver.dart';
import 'package:notes/widgets/json_schema_form.dart';

class KeyMappingsSectionRenderer extends ConfigurationSectionRenderer
    with SchemaResolver {
  const KeyMappingsSectionRenderer();

  (Map<String, dynamic>?, Map<String, dynamic>) _switchProfile(
    Map<String, dynamic> properties,
    dynamic sectionData,
    String activeProfileKey,
  ) {
    final activeProfileSchemaRaw = properties[activeProfileKey];
    final Map<String, dynamic>? activeProfileSchema =
        activeProfileSchemaRaw is Map<String, dynamic>
        ? activeProfileSchemaRaw
        : null;
    final Map<String, dynamic> activeProfileData =
        (sectionData[activeProfileKey] as Map<String, dynamic>?) ?? {};

    return (activeProfileSchema, activeProfileData);
  }

  @override
  bool canRender(String sectionKey, Map<String, dynamic> sectionSchema) =>
      sectionKey == 'key_mappings';

  @override
  Widget buildBody({
    required BuildContext context,
    required String sectionKey,
    required Map<String, dynamic> fullSchema,
    required Map<String, dynamic> sectionSchema,
    required Map<String, dynamic> sectionData,
    required ValueChanged<Map<String, dynamic>> onSectionChanged,
  }) {
    final Map<String, dynamic> properties =
        getProperties(fullSchema, sectionSchema) ?? {};

    final toggleSchemaRaw = properties['is_vim_key_mapping_enabled'];
    final toggleSchema = toggleSchemaRaw is Map<String, dynamic>
        ? resolveSchema(fullSchema, toggleSchemaRaw)
        : null;
    final toggleTitle =
        (toggleSchema?['title'] as String?) ?? 'Vim Key Mappings';
    final toggleDescription = toggleSchema?['description'] as String?;

    final isVimEnabled = sectionData['is_vim_key_mapping_enabled'] == true;
    final activeProfileKey = isVimEnabled
        ? 'vim_profile'
        : 'conventional_profile';

    var (activeProfileSchema, activeProfileData) = _switchProfile(
      properties,
      sectionData,
      activeProfileKey,
    );

    return SingleChildScrollView(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SwitchListTile(
            title: Text(toggleTitle),
            subtitle: toggleDescription != null
                ? Text(toggleDescription)
                : null,
            value: isVimEnabled,
            onChanged: (val) {
              onSectionChanged({
                ...sectionData,
                'is_vim_key_mapping_enabled': val,
              });
            },
          ),
          const SizedBox(height: 16),
          Text(
            isVimEnabled ? 'Vim Profile' : 'Conventional Profile',
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const SizedBox(height: 12),
          if (activeProfileSchema == null)
            const Text('Key mapping profile schema not found')
          else
            JsonSchemaForm(
              key: ValueKey(activeProfileKey),
              schema: fullSchema,
              sectionSchema: activeProfileSchema,
              data: activeProfileData,
              onChanged: (newProfileData) {
                onSectionChanged({
                  ...sectionData,
                  activeProfileKey: newProfileData,
                });
              },
            ),
        ],
      ),
    );
  }
}
