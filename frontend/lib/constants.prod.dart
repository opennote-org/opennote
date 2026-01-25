// Basics
const String baseUrl = "";
const String apiUrl = "/api/v1";

// General
const String backendHealthCheckEndpoint = "$baseUrl$apiUrl/health";
const String retrieveTaskResultEndpoint = "$baseUrl$apiUrl/retrieve_task_result";

// Collection
const String createCollectionEndpoint = "$baseUrl$apiUrl/collections/sync/create_collection";
const String deleteCollectionEndpoint = "$baseUrl$apiUrl/collections/sync/delete_collection";
const String getCollectionEndpoint = "$baseUrl$apiUrl/collections/sync/get_collections";
const String updateCollectionsMetadataEndpoint = "$baseUrl$apiUrl/collections/async/update_collections_metadata";

// Document
const String addDocumentEndpoint = "$baseUrl$apiUrl/documents/async/add_document";
const String importDocumentsEndpoint = "$baseUrl$apiUrl/documents/async/import_documents";
const String deleteDocumentEndpoint = "$baseUrl$apiUrl/documents/async/delete_document";
const String updateDocumentContentEndpoint = "$baseUrl$apiUrl/documents/async/update_document_content";
const String updateDocumentsMetadataEndpoint = "$baseUrl$apiUrl/documents/async/update_documents_metadata";
const String getDocumentContentEndpoint = "$baseUrl$apiUrl/documents/sync/get_document_content";
const String getDocumentMetadataEndpoint = "$baseUrl$apiUrl/documents/sync/get_documents_metadata";

// Search
const String intelligentSearchEndpoint = "$baseUrl$apiUrl/search/sync/intelligent_search";
const String searchEndpoint = "$baseUrl$apiUrl/search/sync/search";
const String reindexEndpoint = "$baseUrl$apiUrl/users/async/reindex";

// User
const String createUserEndpoint = "$baseUrl$apiUrl/users/sync/create_user";
const String loginEndpoint = "$baseUrl$apiUrl/users/sync/login";
const String getUserConfigurationsEndpoint = "$baseUrl$apiUrl/users/sync/get_user_configurations";
const String updateUserConfigurationsEndpoint = "$baseUrl$apiUrl/users/sync/update_user_configurations";
const String getUserConfigurationsSchemarsEndpoint = "$baseUrl$apiUrl/users/sync/get_user_configurations_schemars";

// Backup
const String getBackupsListEndpoint = "$baseUrl$apiUrl/backup/sync/get_backups_list";
const String removeBackupsEndpoint = "$baseUrl$apiUrl/backup/sync/remove_backups";
const String backupEndpoint = "$baseUrl$apiUrl/backup/async/backup";
const String restoreBackupEndpoint = "$baseUrl$apiUrl/backup/async/restore_backup";

// Version
const String frontendVersion = "0.1.0";
