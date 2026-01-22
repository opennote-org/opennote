import 'package:flutter/foundation.dart';
import 'package:notes/services/collection.dart';
import 'package:notes/state/services.dart';

mixin Collections on ChangeNotifier, Services {
  final Map<String, CollectionMetadata> collectionById = {};

  List<CollectionMetadata> get collectionsList =>
      collectionById.values.toList()
        ..sort((a, b) => a.title.compareTo(b.title));
}
