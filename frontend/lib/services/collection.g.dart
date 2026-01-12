// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'collection.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

CollectionMetadata _$CollectionMetadataFromJson(Map<String, dynamic> json) =>
    CollectionMetadata(
      id: json['id'] as String,
      createdAt: json['created_at'] as String,
      lastModified: json['last_modified'] as String,
      title: json['title'] as String,
      documentsMetadataIds: (json['documents_metadata_ids'] as List<dynamic>)
          .map((e) => e as String)
          .toList(),
    );

Map<String, dynamic> _$CollectionMetadataToJson(CollectionMetadata instance) =>
    <String, dynamic>{
      'id': instance.id,
      'created_at': instance.createdAt,
      'last_modified': instance.lastModified,
      'title': instance.title,
      'documents_metadata_ids': instance.documentsMetadataIds,
    };
