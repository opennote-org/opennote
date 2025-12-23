// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'user.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

SearchConfiguration _$SearchConfigurationFromJson(Map<String, dynamic> json) =>
    SearchConfiguration(
      documentChunkSize: (json['document_chunk_size'] as num).toInt(),
      defaultSearchMethod: $enumDecode(
        _$SupportedSearchMethodEnumMap,
        json['default_search_method'],
      ),
      topN: (json['top_n'] as num).toInt(),
    );

Map<String, dynamic> _$SearchConfigurationToJson(
  SearchConfiguration instance,
) => <String, dynamic>{
  'document_chunk_size': instance.documentChunkSize,
  'default_search_method':
      _$SupportedSearchMethodEnumMap[instance.defaultSearchMethod]!,
  'top_n': instance.topN,
};

const _$SupportedSearchMethodEnumMap = {
  SupportedSearchMethod.keyword: 'keyword',
  SupportedSearchMethod.semantic: 'semantic',
};

UserConfigurations _$UserConfigurationsFromJson(Map<String, dynamic> json) =>
    UserConfigurations(
      search: SearchConfiguration.fromJson(
        json['search'] as Map<String, dynamic>,
      ),
    );

Map<String, dynamic> _$UserConfigurationsToJson(UserConfigurations instance) =>
    <String, dynamic>{'search': instance.search};
