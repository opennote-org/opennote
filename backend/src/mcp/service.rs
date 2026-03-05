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
use tokio::sync::Mutex;

use crate::{
    api_models::{document::GetDocumentRequest, search::SearchDocumentRequest},
    app_state::AppState,
    databases::database::filters::{get_collections::GetCollectionFilter, get_documents::GetDocumentFilter},
    documents::document_metadata::DocumentMetadata,
    handlers::{document::get_document_content, search::intelligent_search},
    mcp::{
        requests::{MCPGetCollectionMetadata, MCPSearchDocumentRequest},
        responses::MCPServiceGenericResponse,
    },
    databases::search::SearchScope,
};

#[derive(Clone)]
pub struct MCPService {
    authorization: Arc<Mutex<Option<String>>>,
    tool_router: ToolRouter<Self>,
    app_state: Data<AppState>,
}

#[tool_router]
impl MCPService {
    pub fn new(app_state: Data<AppState>) -> Self {
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

        if let Some(token) = token.as_ref() {
            let resource_ids = match self
                .app_state
                .databases_layer_entry.database
                .get_resource_ids_by_username(token)
                .await
            {
                Ok(ids) => ids,
                Err(e) => {
                    log::warn!("Failed to get resource IDs: {}", e);
                    return Json(MCPServiceGenericResponse { results: None });
                }
            };

            let all_document_metadatas: Vec<DocumentMetadata> = match self
                .app_state
                .databases_layer_entry.database
                .get_documents(&GetDocumentFilter {
                    collection_metadata_ids: resource_ids.clone(),
                    ..Default::default()
                })
                .await
            {
                Ok(metadatas) => metadatas
                    .into_iter()
                    .map(|mut item| {
                        // We don't need the chunk data
                        item.chunks = vec![];
                        item
                    })
                    .collect(),
                Err(e) => {
                    log::warn!("Failed to get all documents: {}", e);
                    return Json(MCPServiceGenericResponse { results: None });
                }
            };

            match serde_json::to_value(all_document_metadatas) {
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

        if let Some(token) = token.as_ref() {
            let is_user_owning_collections = match self
                .app_state
                .databases_layer_entry.database
                .is_user_owning_collections(token, &vec![collection_metadata_id.clone()])
                .await
            {
                Ok(result) => result,
                Err(e) => {
                    log::warn!("Failed to check collection ownership: {}", e);
                    return Json(MCPServiceGenericResponse { results: None });
                }
            };

            let collection_metadata = match self
                .app_state
                .databases_layer_entry.database
                .get_collections(
                    &GetCollectionFilter {
                        ids: vec![collection_metadata_id.clone()],
                        ..Default::default()
                    },
                    false,
                )
                .await
            {
                Ok(result) => result,
                Err(e) => {
                    log::warn!("Failed to get collection metadata: {}", e);
                    return Json(MCPServiceGenericResponse { results: None });
                }
            };

            if is_user_owning_collections {
                return Json(MCPServiceGenericResponse {
                    results: Some(serde_json::to_value(&collection_metadata[0]).unwrap()),
                });
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
            actix_web::web::Json(GetDocumentRequest {
                document_metadata_id,
            }),
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
