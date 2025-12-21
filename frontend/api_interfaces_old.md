# API Documentation

Base URL: `/api/v1`

## General

### Health Check
**GET** `/health`

Returns the health status of the service.

**Response:**
- `200 OK`
```json
{
  "status": "ok",
  "timestamp": "2023-10-27T10:00:00Z"
}
```

### Get Info
**GET** `/info`

Returns information about the service.

**Response:**
- `200 OK`
```json
{
  "service": "Notes Backend",
  "version": "0.1.0",
  "host": "127.0.0.1",
  "port": 8000
}
```

### Retrieve Task Result
**POST** `/retrieve_task_result`

Retrieves the result of an asynchronous task.

**Request Body:**
```json
{
  "task_id": "uuid-string"
}
```

**Success Response:**
- `200 OK`
```json
{
  "task_id": "uuid-string",
  "status": "InProgress" | "Completed",
  "message": null,
  "data": { ... } // Result data if completed
}
```

**Failure Response:**
- `200 OK` (Task Failed)
```json
{
  "task_id": "uuid-string",
  "status": "Failed",
  "message": "Error description",
  "data": null
}
```
- `404 Not Found` (Task ID not found)
```json
{
  "task_id": "uuid-string",
  "status": "Failed",
  "message": "Task not found.",
  "data": null
}
```

## User

### Create User
**POST** `/sync/create_user`

Creates a new user.

**Request Body:**
```json
{
  "username": "user1",
  "password": "password123"
}
```

**Success Response:**
- `200 OK`
```json
{
  "task_id": "",
  "status": "Completed",
  "message": null,
  "data": ""
}
```

**Failure Response:**
- `200 OK`
```json
{
  "task_id": "",
  "status": "Failed",
  "message": "User already exists or other error",
  "data": null
}
```

### Login
**POST** `/sync/login`

Validates user credentials.

**Request Body:**
```json
{
  "username": "user1",
  "password": "password123"
}
```

**Success Response:**
- `200 OK`
```json
{
  "task_id": "",
  "status": "Completed",
  "message": null,
  "data": {
    "is_login": true
  }
}
```

**Failure Response:**
- `200 OK`
```json
{
  "task_id": "",
  "status": "Failed",
  "message": "Invalid credentials",
  "data": null
}
```

## Collection

### Create Collection
**POST** `/sync/create_collection`

Creates a new collection for a user.

**Request Body:**
```json
{
  "collection_title": "My Collection",
  "username": "user1"
}
```

**Success Response:**
- `200 OK`
```json
{
  "task_id": "",
  "status": "Completed",
  "message": null,
  "data": {
    "collection_metadata_id": "uuid-string"
  }
}
```

**Failure Response:**
- `200 OK`
```json
{
  "task_id": "",
  "status": "Failed",
  "message": "Error message",
  "data": null
}
```

### Delete Collection
**POST** `/sync/delete_collection`

Deletes a collection.

**Request Body:**
```json
{
  "collection_metadata_id": "uuid-string"
}
```

**Success Response:**
- `200 OK`
```json
{
  "task_id": "",
  "status": "Completed",
  "message": null,
  "data": {
      "metadata_id": "uuid-string",
      "created_at": "timestamp",
      "last_modified": "timestamp",
      "title": "My Collection",
      "documents_metadata_ids": []
  }
}
```

**Failure Response:**
- `200 OK`
```json
{
  "task_id": "",
  "status": "Failed",
  "message": "Collection metadata id was not found.",
  "data": null
}
```

### Get Collections
**GET** `/sync/get_collections`

Retrieves all collections.

**Query Parameters:**
- `username`: The username to fetch collections for.

**Response:**
- `200 OK`
```json
{
  "task_id": "",
  "status": "Completed",
  "message": null,
  "data": [
    {
      "metadata_id": "uuid-string",
      "created_at": "timestamp",
      "last_modified": "timestamp",
      "title": "My Collection",
      "documents_metadata_ids": []
    }
  ]
}
```

### Update Collections Metadata
**POST** `/async/update_collections_metadata`

Updates metadata for multiple collections.

**Note:** Immutable fields (`created_at`, `last_modified`, `documents_metadata_ids`) must be empty/default in the request.

**Request Body:**
```json
{
  "collection_metadatas": [
    {
      "metadata_id": "uuid-string",
      "created_at": "",
      "last_modified": "",
      "title": "New Title",
      "documents_metadata_ids": []
    }
  ]
}
```

**Success Response:**
- `200 OK`
```json
{
  "task_id": "",
  "status": "Completed",
  "message": null,
  "data": {}
}
```

**Failure Response:**
- `200 OK`
```json
{
  "task_id": "",
  "status": "Failed",
  "message": "Error message",
  "data": null
}
```

## Document

### Add Document (Async)
**POST** `/async/add_document`

Adds a new document asynchronously.

**Request Body:**
```json
{
  "collection_metadata_id": "uuid-string",
  "title": "My Document",
  "content": "Document content..."
}
```

**Response:**
- `200 OK`
```json
{
  "task_id": "uuid-string",
  "status": "InProgress",
  "message": null,
  "data": null
}
```
Use `retrieve_task_result` with the returned `task_id` to get the result.

**Task Result (Success):**
```json
{
  "document_metadata_id": "uuid-string"
}
```

**Task Result (Failure):**
Check `retrieve_task_result` response for `status: "Failed"` and `message`.

### Import Documents (Async)
**POST** `/async/import_documents`

Imports documents from external sources asynchronously.

**Request Body:**
```json
{
  "collection_metadata_id": "uuid-string",
  "imports": [
    {
      "import_type": "Webpage",
      "artifact": "https://example.com"
    },
    {
      "import_type": "TextFile",
      "artifact": "Content of the text file..."
    },
    {
      "import_type": "RelationshipDatabase",
      "artifact": {
        "database_type": "mysql", // aslo supports `postgres` and `sqlite`
        "username": "user",
        "password": "password",
        "host": "localhost",
        "port": "3306",
        "database_name": "db_name" or null,
        "query": "SELECT * FROM table",
        "column_to_fetch": "content_column",
        "table_name": "table" or null
      }
    }
  ]
}
```

**Response:**
- `200 OK`
```json
{
  "task_id": "uuid-string",
  "status": "InProgress",
  "message": null,
  "data": null
}
```
Use `retrieve_task_result` with the returned `task_id` to get the result.

**Task Result (Success):**
Returns a list of failed imports and a list of successfully created document metadata IDs.

Example of successful import (Full Response from `retrieve_task_result`):
```json
{
  "task_id": "uuid-string",
  "status": "Completed",
  "message": null,
  "data": {
    "failed_import_tasks": [],
    "document_metadata_ids": ["uuid-string-1", "uuid-string-2"]
  }
}
```

Example with failed imports (Full Response from `retrieve_task_result`):
```json
{
  "task_id": "uuid-string",
  "status": "Completed",
  "message": null,
  "data": {
    "failed_import_tasks": [
      {
        "import_type": "Webpage",
        "artifact": "https://broken-link.com"
      }
    ],
    "document_metadata_ids": ["uuid-string-1"]
  }
}
```

**Task Result (Failure):**
Example of a task failure (e.g. system error):
```json
{
  "task_id": "uuid-string",
  "status": "Failed",
  "message": "Detailed error message describing what went wrong",
  "data": null
}
```

### Delete Document (Async)
**POST** `/async/delete_document`

Deletes a document asynchronously.

**Request Body:**
```json
{
  "document_metadata_id": "uuid-string"
}
```

**Response:**
- `200 OK` (InProgress)

**Task Result (Success):**
```json
{
  "document_metadata_id": "uuid-string"
}
```

**Task Result (Failure):**
Check `retrieve_task_result` response for `status: "Failed"` and `message`.

### Update Document Content (Async)
**POST** `/async/update_document_content`

Updates a document's content asynchronously.

**Request Body:**
```json
{
  "document_metadata_id": "uuid-string",
  "collection_metadata_id": "uuid-string",
  "title": "New Title",
  "content": "New content..."
}
```

**Response:**
- `200 OK` (InProgress)

**Task Result (Success):**
```json
{
  "document_metadata_id": "uuid-string"
}
```

**Task Result (Failure):**
Check `retrieve_task_result` response for `status: "Failed"` and `message`.

### Update Documents Metadata
**POST** `/async/update_documents_metadata`

Updates metadata for multiple documents.

**Note:** Immutable fields (`created_at`, `last_modified`, `chunks`) must be empty/default in the request. Changing `collection_metadata_id` will move the document to the new collection.

**Request Body:**
```json
{
  "document_metadatas": [
    {
      "metadata_id": "uuid-string",
      "created_at": "",
      "last_modified": "",
      "collection_metadata_id": "uuid-string",
      "title": "New Title",
      "chunks": []
    }
  ]
}
```

**Success Response:**
- `200 OK`
```json
{
  "task_id": "",
  "status": "Completed",
  "message": null,
  "data": {}
}
```

**Failure Response:**
- `200 OK`
```json
{
  "task_id": "",
  "status": "Failed",
  "message": "Error message",
  "data": null
}
```

### Get Document Content
**POST** `/sync/get_document_content`

Retrieves document content (chunks).

**Request Body:**
```json
{
  "document_metadata_id": "uuid-string"
}
```

**Success Response:**
- `200 OK`
```json
{
  "task_id": "",
  "status": "Completed",
  "message": null,
  "data": [
    {
      "id": "chunk-uuid",
      "document_metadata_id": "uuid-string",
      "collection_metadata_id": "uuid-string",
      "content": "chunk content"
    }
  ]
}
```

**Failure Response:**
- `200 OK`
```json
{
  "task_id": "",
  "status": "Failed",
  "message": "Failed to get the document",
  "data": null
}
```

### Get Documents Metadata
**GET** `/sync/get_documents_metadata`

Retrieves metadata for all documents.

**Query Parameters:**
- `collection_metadata_id`: The collection metadata id to fetch documents for.

**Response:**
- `200 OK`
```json
{
  "task_id": "",
  "status": "Completed",
  "message": null,
  "data": [
    {
      "metadata_id": "uuid-string",
      "created_at": "timestamp",
      "last_modified": "timestamp",
      "collection_metadata_id": "uuid-string",
      "title": "My Document",
      "chunks": ["chunk-uuid-1", "chunk-uuid-2"]
    }
  ]
}
```

## Search

### Intelligent Search
**POST** `/sync/intelligent_search`

Performs semantic search on documents.

**Request Body:**
```json
{
  "query": "search query",
  "top_n": 5,
  "scope": {
    "search_scope": "Document" | "Collection" | "Userspace",
    "id": "uuid-string or username"
  }
}
```

**Success Response:**
- `200 OK`
```json
{
  "task_id": "",
  "status": "Completed",
  "message": null,
  "data": [
    {
      "document_chunk": {
        "id": "chunk-uuid",
        "document_metadata_id": "uuid-string",
        "collection_metadata_id": "uuid-string",
        "content": "matching content"
      },
      "score": 0.85
    }
  ]
}
```

**Failure Response:**
- `200 OK`
```json
{
  "task_id": "",
  "status": "Failed",
  "message": "Failed to talk to the database. Please check the connection.",
  "data": null
}
```

### Full Text Search
**POST** `/sync/search`

Performs full-text search on documents.

**Request Body:**
```json
{
  "query": "search query",
  "top_n": 5,
  "scope": {
    "search_scope": "Document" | "Collection" | "Userspace",
    "id": "uuid-string or username"
  }
}
```

**Success Response:**
- `200 OK`
```json
{
  "task_id": "",
  "status": "Completed",
  "message": null,
  "data": [
    {
      "document_chunk": {
        "id": "chunk-uuid",
        "document_metadata_id": "uuid-string",
        "collection_metadata_id": "uuid-string",
        "content": "matching content"
      },
      "score": 0.85
    }
  ]
}
```

**Failure Response:**
- `200 OK`
```json
{
  "task_id": "",
  "status": "Failed",
  "message": "Failed to talk to the database. Please check the connection.",
  "data": null
}
```
