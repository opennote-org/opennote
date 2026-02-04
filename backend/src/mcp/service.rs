use std::sync::Arc;

use actix_web::{
    body::MessageBody,
    web::{Buf, Data},
};
use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters}, model::{InitializeRequestParams, InitializeResult, ServerCapabilities, ServerInfo}, service::RequestContext, tool, tool_handler, tool_router, ErrorData, Json, RoleServer, ServerHandler
};
use rmcp_actix_web::transport::AuthorizationHeader;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{Mutex, RwLock};

use crate::{
    api_models::search::SearchDocumentRequest,
    app_state::AppState,
    documents::{
        document_chunk::DocumentChunkSearchResult, operations::retrieve_document_ids_by_scope,
    },
    handlers::search::intelligent_search,
    identities::user,
    search::{SearchScopeIndicator, build_search_results, semantic::search_documents_semantically},
    utilities::acquire_data,
};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MCPSearchRequest {
    #[schemars(description = "keywords, phrases or sentences you may want to search")]
    pub search_phrase: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct MCPServiceGenericResponse {
    #[schemars(description = "results")]
    pub results: Option<Value>,
}

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

    #[tool(description = "Semantically search the OpenNote documents")]
    pub async fn semantic_search(
        &self,
        Parameters(MCPSearchRequest { search_phrase }): Parameters<MCPSearchRequest>,
    ) -> Json<MCPServiceGenericResponse> {
        let token = self.authorization.lock().await;

        if let Some(token) = token.as_ref() {
            dbg!(token);
            match intelligent_search(
                self.app_state.clone(),
                actix_web::web::Json(SearchDocumentRequest {
                    query: search_phrase,
                    top_n: 20,
                    scope: SearchScopeIndicator {
                        search_scope: crate::search::SearchScope::Userspace,
                        id: token.clone(),
                    },
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
