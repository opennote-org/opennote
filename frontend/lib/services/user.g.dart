// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'user.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

SearchConfiguration _$SearchConfigurationFromJson(Map<String, dynamic> json) =>
    SearchConfiguration(
      documentChunkSize: (json['document_chunk_size'] as num).toInt(),
    );

Map<String, dynamic> _$SearchConfigurationToJson(
  SearchConfiguration instance,
) => <String, dynamic>{'document_chunk_size': instance.documentChunkSize};

UserConfigurations _$UserConfigurationsFromJson(Map<String, dynamic> json) =>
    UserConfigurations(
      search: SearchConfiguration.fromJson(
        json['search'] as Map<String, dynamic>,
      ),
    );

Map<String, dynamic> _$UserConfigurationsToJson(UserConfigurations instance) =>
    <String, dynamic>{'search': instance.search};
