// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'tabs.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

SavedTabsStates _$SavedTabsStatesFromJson(Map<String, dynamic> json) =>
    SavedTabsStates(
      activeObject: ActiveObject.fromJson(
        json['activeObject'] as Map<String, dynamic>,
      ),
      openObjectIds: (json['openObjectIds'] as List<dynamic>)
          .map((e) => e as String)
          .toList(),
      lastActiveObjectId: json['lastActiveObjectId'] as String?,
    );

Map<String, dynamic> _$SavedTabsStatesToJson(SavedTabsStates instance) =>
    <String, dynamic>{
      'activeObject': instance.activeObject,
      'openObjectIds': instance.openObjectIds,
      'lastActiveObjectId': instance.lastActiveObjectId,
    };

ActiveObject _$ActiveObjectFromJson(Map<String, dynamic> json) => ActiveObject(
  $enumDecode(_$ActiveObjectTypeEnumMap, json['type']),
  json['id'] as String?,
);

Map<String, dynamic> _$ActiveObjectToJson(ActiveObject instance) =>
    <String, dynamic>{
      'type': _$ActiveObjectTypeEnumMap[instance.type]!,
      'id': instance.id,
    };

const _$ActiveObjectTypeEnumMap = {
  ActiveObjectType.collection: 'collection',
  ActiveObjectType.document: 'document',
  ActiveObjectType.none: 'none',
};
