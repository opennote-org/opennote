use actix_web::web::Data;
use rmcp::{
    Json, ServerHandler, handler::server::{tool::ToolRouter, wrapper::Parameters}, model::{ServerCapabilities, ServerInfo}, tool, tool_handler, tool_router
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::{runtime::Handle, sync::RwLock};

use crate::{
    app_state::AppState,
    documents::{
        document_chunk::DocumentChunkSearchResult, operations::retrieve_document_ids_by_scope,
    },
    search::{build_search_results, semantic::search_documents_semantically},
    utilities::acquire_data,
};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MCPSearchRequest {
    #[schemars(description = "keywords, phrases or sentences you may want to search")]
    pub search_phrase: String,

    #[schemars(description = "username of the user. you may ask him or her")]
    pub username: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct MCPSearchResponse {
    #[schemars(description = "search results")]
    pub results: Vec<Value>,
}

#[derive(Clone)]
pub struct MCPSearchService {
    tool_router: ToolRouter<Self>,
    app_state: Data<RwLock<AppState>>,
}

#[tool_router]
impl MCPSearchService {
    pub fn new(app_state: Data<RwLock<AppState>>) -> Self {
        Self {
            tool_router: Self::tool_router(),
            app_state,
        }
    }

    #[tool(description = "Semantically search the OpenNote documents")]
    pub fn semantic_search(
        &self,
        Parameters(MCPSearchRequest {
            search_phrase,
            username,
        }): Parameters<MCPSearchRequest>,
    ) -> Json<MCPSearchResponse> {
        let handle = Handle::current();
        let (index_name, db_client, metadata_storage, _, config, identities_storage, _) =
            handle.block_on(acquire_data(&self.app_state));

        let document_metadata_ids: Vec<String> = retrieve_document_ids_by_scope(
            &mut metadata_storage.blocking_lock(),
            &mut identities_storage.blocking_lock(),
            crate::search::SearchScope::Userspace,
            &username,
        );

        match handle.block_on(search_documents_semantically(
            &db_client,
            document_metadata_ids,
            &index_name,
            &search_phrase,
            20,
            &config.embedder.provider,
            &config.embedder.base_url,
            &config.embedder.api_key,
            &config.embedder.model,
            &config.embedder.encoding_format,
        )) {
            Ok(results) => {
                let metadata_storage = metadata_storage.blocking_lock();
                let results: Vec<DocumentChunkSearchResult> = build_search_results(
                    Some(results),
                    None,
                    &metadata_storage.collections,
                    &metadata_storage.documents,
                );

                return Json(MCPSearchResponse {
                    results: results
                        .into_iter()
                        .map(|item| serde_json::to_value(item).unwrap())
                        .collect(),
                });
            }
            Err(error) => {
                log::error!("Failed when trying searching: {}", error);
                return Json(MCPSearchResponse { results: vec![] });
            }
        };
    }
}

#[tool_handler]
impl ServerHandler for MCPSearchService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Offer access to user's OpenNote".into()),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            ..Default::default()
        }
    }
}