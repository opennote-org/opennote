// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'search.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

DocumentChunkSearchResult _$DocumentChunkSearchResultFromJson(
  Map<String, dynamic> json,
) => DocumentChunkSearchResult(
  documentChunk: DocumentChunk.fromJson(
    json['document_chunk'] as Map<String, dynamic>,
  ),
  score: (json['score'] as num).toDouble(),
);

Map<String, dynamic> _$DocumentChunkSearchResultToJson(
  DocumentChunkSearchResult instance,
) => <String, dynamic>{
  'document_chunk': instance.documentChunk,
  'score': instance.score,
};
