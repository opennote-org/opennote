// Basics
const String baseUrl = "";
const String apiUrl = "/api/v1";

// General
const String backendHealthCheckEndpoint = "$baseUrl$apiUrl/health";
const String retrieveTaskResultEndpoint = "$baseUrl$apiUrl/retrieve_task_result";

// Collection
const String createCollectionEndpoint = "$baseUrl$apiUrl/sync/create_collection";
const String deleteCollectionEndpoint = "$baseUrl$apiUrl/sync/delete_collection";
const String getCollectionEndpoint = "$baseUrl$apiUrl/sync/get_collections";
const String updateCollectionsMetadataEndpoint = "$baseUrl$apiUrl/async/update_collections_metadata";

// Document
const String addDocumentEndpoint = "$baseUrl$apiUrl/async/add_document";
const String importDocumentsEndpoint = "$baseUrl$apiUrl/async/import_documents";
const String deleteDocumentEndpoint = "$baseUrl$apiUrl/async/delete_document";
const String updateDocumentContentEndpoint = "$baseUrl$apiUrl/async/update_document_content";
const String updateDocumentsMetadataEndpoint = "$baseUrl$apiUrl/async/update_documents_metadata";
const String getDocumentContentEndpoint = "$baseUrl$apiUrl/sync/get_document_content";
const String getDocumentMetadataEndpoint = "$baseUrl$apiUrl/sync/get_documents_metadata";

// Search
const String intelligentSearchEndpoint = "$baseUrl$apiUrl/sync/intelligent_search";
const String searchEndpoint = "$baseUrl$apiUrl/sync/search";

// User
const String createUserEndpoint = "$baseUrl$apiUrl/sync/create_user";
const String loginEndpoint = "$baseUrl$apiUrl/sync/login";
const String getUserConfigurationsEndpoint = "$baseUrl$apiUrl/sync/get_user_configurations";
const String updateUserConfigurationsEndpoint = "$baseUrl$apiUrl/sync/update_user_configurations";