import 'package:dio/dio.dart';
import 'package:json_annotation/json_annotation.dart';
import 'package:notes/constants.dart';

part 'document.g.dart';

@JsonSerializable(fieldRename: FieldRename.snake)
class DocumentChunk {
  String id;
  String documentMetadataId;
  String collectionMetadataId;
  String content;

  DocumentChunk({
    required this.id,
    required this.documentMetadataId,
    required this.collectionMetadataId,
    required this.content,
  });

  factory DocumentChunk.fromJson(Map<String, dynamic> json) =>
      _$DocumentChunkFromJson(json);

  Map<String, dynamic> toJson() => _$DocumentChunkToJson(this);
}

@JsonSerializable(fieldRename: FieldRename.snake)
class DocumentMetadata {
  String id;
  String createdAt;
  String lastModified;
  String collectionMetadataId;
  String title;
  List<String> chunks;

  DocumentMetadata({
    required this.id,
    required this.createdAt,
    required this.lastModified,
    required this.collectionMetadataId,
    required this.title,
    required this.chunks,
  });

  factory DocumentMetadata.fromJson(Map<String, dynamic> json) =>
      _$DocumentMetadataFromJson(json);
  Map<String, dynamic> toJson() => _$DocumentMetadataToJson(this);

  bool isLocalDocument() {
    if (id.startsWith('temp_doc_')) {
      return true;
    }

    return false;
  }
}

class DocumentManagementService {
  Future<String> addDocument(
    Dio dio,
    String username,
    String title,
    String collectionMetadataId,
    String content,
  ) async {
    final response = await dio.post(
      addDocumentEndpoint,
      data: {
        "username": username,
        "collection_metadata_id": collectionMetadataId,
        "title": title,
        "content": content,
      },
    );
    return response.data!["task_id"];
  }

  Future<String> importDocuments(
    Dio dio,
    String username,
    String collectionMetadataId,
    List<Map<String, dynamic>> imports,
  ) async {
    final response = await dio.post(
      importDocumentsEndpoint,
      data: {
        "username": username,
        "collection_metadata_id": collectionMetadataId,
        "imports": imports,
      },
    );
    return response.data!["task_id"];
  }

  Future<String> deleteDocument(Dio dio, String documentMetadataId) async {
    final response = await dio.post(
      deleteDocumentEndpoint,
      data: {"document_metadata_id": documentMetadataId},
    );
    return response.data!["task_id"];
  }

  Future<String> updateDocumentContent(
    Dio dio,
    String username,
    String documentMetadataId,
    String collectionMetadataId,
    String title,
    String content,
  ) async {
    final response = await dio.post(
      updateDocumentContentEndpoint,
      data: {
        "username": username,
        "document_metadata_id": documentMetadataId,
        "collection_metadata_id": collectionMetadataId,
        "title": title,
        "content": content,
      },
    );
    return response.data!["task_id"];
  }

  Future<String> updateDocumentsMetadata(
    Dio dio,
    List<DocumentMetadata> documents,
  ) async {
    final List<Map<String, dynamic>> data = documents.map((e) {
      return {
        "metadata_id": e.id,
        "created_at": "",
        "last_modified": "",
        "collection_metadata_id": e.collectionMetadataId,
        "title": e.title,
        "chunks": [],
      };
    }).toList();

    final response = await dio.post(
      updateDocumentsMetadataEndpoint,
      data: {"document_metadatas": data},
    );
    return response.data!["task_id"];
  }

  Future<List<DocumentMetadata>> getDocumentsMetadata(
    Dio dio,
    String collectionMetadataId,
  ) async {
    final response = await dio.get(
      getDocumentMetadataEndpoint,
      queryParameters: {"collection_metadata_id": collectionMetadataId},
    );
    final List<dynamic> data = response.data!["data"] as List<dynamic>;

    return data
        .map((e) => DocumentMetadata.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  Future<List<DocumentChunk>> getDocument(
    Dio dio,
    String documentMetadataId,
  ) async {
    final response = await dio.post(
      getDocumentContentEndpoint,
      data: {"document_metadata_id": documentMetadataId},
    );

    final List<dynamic> data = response.data!["data"] as List<dynamic>;
    return data
        .map((chunk) => DocumentChunk.fromJson(chunk as Map<String, dynamic>))
        .toList();
  }

  Future<String> reindex(Dio dio, String username) async {
    final response = await dio.post(
      reindexEndpoint,
      data: {"username": username},
    );
    return response.data!["task_id"];
  }
}
