import 'package:dio/dio.dart';
import 'package:json_annotation/json_annotation.dart';
import 'package:notes/constants.dart';

part 'collection.g.dart';

@JsonSerializable(fieldRename: FieldRename.snake)
class CollectionMetadata {
  String id;
  String createdAt;
  String lastModified;
  String title;
  List<String> documentsMetadataIds;

  CollectionMetadata({
    required this.id,
    required this.createdAt,
    required this.lastModified,
    required this.title,
    required this.documentsMetadataIds,
  });

  factory CollectionMetadata.fromJson(Map<String, dynamic> json) => _$CollectionMetadataFromJson(json);
  Map<String, dynamic> toJson() => _$CollectionMetadataToJson(this);
}

class CollectionManagementService {
  Future<String> createCollection(Dio dio, String title, String username) async {
    final response = await dio.post(createCollectionEndpoint, data: {"collection_title": title, "username": username});
    return response.data!["data"]["collection_metadata_id"];
  }

  Future<String> deleteCollection(Dio dio, String collectionMetadataId) async {
    final response = await dio.post(deleteCollectionEndpoint, data: {"collection_metadata_id": collectionMetadataId});
    
    if (response.data?["data"] == null) {
      throw Exception("Invalid response: missing 'data' field");
    }
    
    final metadata = CollectionMetadata.fromJson(response.data!["data"] as Map<String, dynamic>);
    
    return metadata.id;
  }

  Future<List<CollectionMetadata>> getCollections(Dio dio, String username) async {
    final response = await dio.get(getCollectionEndpoint, queryParameters: {"username": username});
    final List<dynamic> items = response.data!["data"] as List<dynamic>;

    return items.map((e) => CollectionMetadata.fromJson(e as Map<String, dynamic>)).toList();
  }

  Future<String> updateCollectionsMetadata(Dio dio, List<CollectionMetadata> collections) async {
    final List<Map<String, dynamic>> data = collections.map((e) {
      return {"id": e.id, "created_at": "", "last_modified": "", "title": e.title, "documents_metadata_ids": []};
    }).toList();

    final response = await dio.post(updateCollectionsMetadataEndpoint, data: {"collection_metadatas": data});
    return response.data!["task_id"];
  }
}
