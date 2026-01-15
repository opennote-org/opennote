mixin SchemaResolver {
  Map<String, dynamic> resolveRef(Map<String, dynamic> rootSchema, String ref) {
    final parts = ref.split('/');
    if (parts.isEmpty || parts[0] != '#') return {};
    dynamic current = rootSchema;
    for (int i = 1; i < parts.length; i++) {
      current = current[parts[i]];
      if (current == null) return {};
    }
    return current is Map<String, dynamic> ? current : {};
  }

  Map<String, dynamic> resolveSchema(
    Map<String, dynamic> rootSchema,
    Map<String, dynamic> schema,
  ) {
    if (schema.containsKey('\$ref')) {
      final resolved = resolveRef(rootSchema, schema['\$ref'] as String);
      return {...resolved, ...schema};
    }
    return schema;
  }

  Map<String, dynamic>? getProperties(
    Map<String, dynamic> rootSchema,
    Map<String, dynamic> schema,
  ) {
    final resolved = resolveSchema(rootSchema, schema);
    return resolved['properties'] as Map<String, dynamic>?;
  }
}

