// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'document.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

DocumentChunk _$DocumentChunkFromJson(Map<String, dynamic> json) =>
    DocumentChunk(
      id: json['id'] as String,
      documentMetadataId: json['document_metadata_id'] as String,
      collectionMetadataId: json['collection_metadata_id'] as String,
      content: json['content'] as String,
    );

Map<String, dynamic> _$DocumentChunkToJson(DocumentChunk instance) =>
    <String, dynamic>{
      'id': instance.id,
      'document_metadata_id': instance.documentMetadataId,
      'collection_metadata_id': instance.collectionMetadataId,
      'content': instance.content,
    };

DocumentMetadata _$DocumentMetadataFromJson(Map<String, dynamic> json) =>
    DocumentMetadata(
      id: json['id'] as String,
      createdAt: json['created_at'] as String,
      lastModified: json['last_modified'] as String,
      collectionMetadataId: json['collection_metadata_id'] as String,
      title: json['title'] as String,
      chunks: (json['chunks'] as List<dynamic>)
          .map((e) => e as String)
          .toList(),
    );

Map<String, dynamic> _$DocumentMetadataToJson(DocumentMetadata instance) =>
    <String, dynamic>{
      'id': instance.id,
      'created_at': instance.createdAt,
      'last_modified': instance.lastModified,
      'collection_metadata_id': instance.collectionMetadataId,
      'title': instance.title,
      'chunks': instance.chunks,
    };
