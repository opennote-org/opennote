import 'package:dio/dio.dart';
import 'package:json_annotation/json_annotation.dart';
import 'package:notes/constants.dart';
import 'package:notes/services/document.dart';

part 'search.g.dart';

enum SearchScope { document, collection, userspace }

@JsonSerializable(fieldRename: FieldRename.snake)
class DocumentChunkSearchResult {
  DocumentChunk documentChunk;
  double score;

  DocumentChunkSearchResult({
    required this.documentChunk,
    required this.score,
  });

  factory DocumentChunkSearchResult.fromJson(Map<String, dynamic> json) => _$DocumentChunkSearchResultFromJson(json);

  Map<String, dynamic> toJson() => _$DocumentChunkSearchResultToJson(this);
}

class SearchService {
  Future<List<DocumentChunkSearchResult>> intelligentSearch(
    Dio dio, {
    required String query,
    required SearchScope scope,
    required String scopeId,
    int topN = 10,
  }) async {
    final response = await dio.post(
      intelligentSearchEndpoint,
      data: {
        "query": query,
        "top_n": topN,
        "scope": {"search_scope": scope.name, "id": scopeId},
      },
    );
    final List<dynamic> chunksJson = response.data!["data"];
    return chunksJson.map((json) => DocumentChunkSearchResult.fromJson(json)).toList();
  }

  Future<List<DocumentChunkSearchResult>> keywordSearch(
    Dio dio, {
    required String query,
    required SearchScope scope,
    required String scopeId,
    int topN = 10,
  }) async {
    final response = await dio.post(
      searchEndpoint,
      data: {
        "query": query,
        "top_n": topN,
        "scope": {"search_scope": scope.name, "id": scopeId},
      },
    );
    final List<dynamic> chunksJson = response.data!["data"];
    return chunksJson.map((json) => DocumentChunkSearchResult.fromJson(json)).toList();
  }
}
