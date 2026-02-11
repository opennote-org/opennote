use std::sync::Arc;

use actix_web::{
    body::MessageBody,
    web::{Buf, Data},
};
use rmcp::{
    ErrorData, Json, RoleServer, ServerHandler,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{InitializeRequestParams, InitializeResult, ServerCapabilities, ServerInfo},
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use rmcp_actix_web::transport::AuthorizationHeader;
use serde_json::Value;
use tokio::sync::{Mutex, RwLock};

use crate::{
    api_models::{document::GetDocumentRequest, search::SearchDocumentRequest},
    app_state::AppState,
    documents::document_metadata::DocumentMetadata,
    handlers::{document::get_document_content, search::intelligent_search},
    mcp::{requests::{MCPGetCollectionMetadata, MCPSearchDocumentRequest}, responses::MCPServiceGenericResponse},
    search::SearchScope,
    utilities::acquire_data,
};

#[derive(Clone)]
pub struct MCPService {
    authorization: Arc<Mutex<Option<String>>>,
    tool_router: ToolRouter<Self>,
    app_state: Data<RwLock<AppState>>,
}

#[tool_router]
impl MCPService {
    pub fn new(app_state: Data<RwLock<AppState>>) -> Self {
        Self {
            tool_router: Self::tool_router(),
            authorization: Arc::new(Mutex::new(None)),
            app_state,
        }
    }

    #[tool(description = "Semantically search the user's OpenNote documents")]
    pub async fn semantic_search(
        &self,
        Parameters(MCPSearchDocumentRequest {
            query,
            top_n,
            mut scope,
        }): Parameters<MCPSearchDocumentRequest>,
    ) -> Json<MCPServiceGenericResponse> {
        let token = self.authorization.lock().await;

        if let Some(token) = token.as_ref() {
            if scope.search_scope == SearchScope::Userspace {
                scope.id = token.to_string();
            }

            match intelligent_search(
                self.app_state.clone(),
                actix_web::web::Json(SearchDocumentRequest {
                    query,
                    top_n,
                    scope,
                }),
            )
            .await
            {
                Ok(result) => {
                    let result = result.into_body().try_into_bytes().unwrap().reader();
                    let value: Value = serde_json::from_reader(result).unwrap();
                    return Json(MCPServiceGenericResponse {
                        results: Some(value),
                    });
                }
                Err(error) => {
                    log::warn!("MCP service reported error: {}", error);
                    return Json(MCPServiceGenericResponse { results: None });
                }
            }
        }

        Json(MCPServiceGenericResponse { results: None })
    }

    #[tool(description = "Get metadatas of the user's OpenNote documents")]
    pub async fn get_all_user_documents_metadatas(&self) -> Json<MCPServiceGenericResponse> {
        let token = self.authorization.lock().await;
        let (_, metadata_storage, _, _, identities_storage, _) =
            acquire_data(&self.app_state).await;

        if let Some(token) = token.as_ref() {
            let identities_storage = identities_storage.lock().await;
            let metadata_storage = metadata_storage.lock().await;

            let resource_ids = identities_storage.get_resource_ids_by_username(token);

            let documents_metadatas_list: Vec<DocumentMetadata> = metadata_storage
                .documents
                .iter()
                .filter(|(_, metadata)| resource_ids.contains(&&metadata.collection_metadata_id))
                .map(|(_, metadata)| {
                    let mut metadata = metadata.to_owned();
                    metadata.chunks = vec![];
                    metadata
                })
                .collect();

            match serde_json::to_value(documents_metadatas_list) {
                Ok(result) => {
                    return Json(MCPServiceGenericResponse {
                        results: Some(result),
                    });
                }
                Err(error) => {
                    log::warn!("Failed to serialize document metadatas: {}", error);
                    return Json(MCPServiceGenericResponse { results: None });
                }
            }
        }

        Json(MCPServiceGenericResponse { results: None })
    }

    #[tool(description = "Get metadatas of a collection")]
    pub async fn get_collection_metadata(
        &self,
        Parameters(MCPGetCollectionMetadata {
            collection_metadata_id,
        }): Parameters<MCPGetCollectionMetadata>,
    ) -> Json<MCPServiceGenericResponse> {
        let token = self.authorization.lock().await;
        let (_, metadata_storage, _, _, identities_storage, _) =
            acquire_data(&self.app_state).await;

        if let Some(token) = token.as_ref() {
            let identities_storage = identities_storage.lock().await;
            let metadata_storage = metadata_storage.lock().await;

            if identities_storage
                .is_user_owning_collections(token, &vec![collection_metadata_id.clone()])
            {
                if let Some(metadata) = metadata_storage.collections.get(&collection_metadata_id) {
                    match serde_json::to_value(metadata) {
                        Ok(result) => {
                            return Json(MCPServiceGenericResponse {
                                results: Some(result),
                            });
                        }
                        Err(error) => {
                            log::warn!("Failed to serialize document metadatas: {}", error);
                            return Json(MCPServiceGenericResponse { results: None });
                        }
                    }
                }
            }
        }

        Json(MCPServiceGenericResponse { results: None })
    }
    
    /// TODO: 
    /// - check document ownership before returning a document
    /// - enforce authentication bearer in all endpoints, not just MCP server
    #[tool(description = "Get document content by supplying a document metadata id")]
    pub async fn get_document_content_by_document_metadata_id(
        &self,
        Parameters(GetDocumentRequest {
            document_metadata_id,
        }): Parameters<GetDocumentRequest>,
    ) -> Json<MCPServiceGenericResponse> {
        match get_document_content(
            self.app_state.clone(),
            actix_web::web::Json(GetDocumentRequest { document_metadata_id }),
        )
        .await
        {
            Ok(result) => {
                let result = result.into_body().try_into_bytes().unwrap().reader();
                let value: Value = serde_json::from_reader(result).unwrap();
                Json(MCPServiceGenericResponse {
                    results: Some(value),
                })
            }
            Err(error) => {
                log::warn!("Failed to get document content: {}", error);
                Json(MCPServiceGenericResponse { results: None })
            }
        }
    }
}

#[tool_handler]
impl ServerHandler for MCPService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Offer access to user's OpenNote. A notebook of the user.".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }

    async fn initialize(
        &self,
        request: InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, ErrorData> {
        // Store peer info
        if context.peer.peer_info().is_none() {
            context.peer.set_peer_info(request);
        }

        // Extract and store Authorization header if present
        if let Some(auth) = context.extensions.get::<AuthorizationHeader>() {
            let mut stored_auth = self.authorization.lock().await;
            let stripped = auth.0.strip_prefix("Bearer ");

            if let Some(stripped) = stripped {
                *stored_auth = Some(stripped.to_owned());
                log::info!("Authorization header found");
            }
        }

        log::info!("No Authorization header found - proxy calls will fail");

        Ok(self.get_info())
    }
}
