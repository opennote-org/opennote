use std::sync::Arc;

use anyhow::Result;
use rmcp::{
    ErrorData, Json, RoleServer, ServerHandler,
    handler::server::wrapper::Parameters,
    model::{InitializeRequestParams, InitializeResult, ServerCapabilities, ServerInfo},
    service::RequestContext,
    tool, tool_handler, tool_router,
};

use crate::{
    instructions::INSTRUCTIONS,
    requests::{MCPReadBlocksRequest, MCPSearchRequest},
    responses::MCPServiceGenericResponse,
    traits::OpenNoteMCPServiceImplementation,
};

#[derive(Clone)]
pub struct OpenNoteMCPService {
    mcp_implementation: Arc<dyn OpenNoteMCPServiceImplementation>,
}

#[tool_router]
impl OpenNoteMCPService {
    pub fn new(mcp_implementation: Arc<dyn OpenNoteMCPServiceImplementation>) -> Self {
        Self { mcp_implementation }
    }

    #[tool(
        description = "search the user's OpenNote documents. you should read blocks to get block_ids before making a search"
    )]
    pub async fn search(
        &self,
        Parameters(MCPSearchRequest {
            search_method,
            block_ids,
            query,
            top_n,
        }): Parameters<MCPSearchRequest>,
    ) -> Json<MCPServiceGenericResponse> {
        match self
            .mcp_implementation
            .search(MCPSearchRequest {
                search_method,
                block_ids,
                query,
                top_n,
            })
            .await
        {
            Ok(result) => {
                let value = serde_json::to_value(result).unwrap();
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

    #[tool(description = "read user's OpenNote blocks")]
    pub async fn read_blocks(
        &self,
        Parameters(MCPReadBlocksRequest {
            block_ids,
            has_payload,
        }): Parameters<MCPReadBlocksRequest>,
    ) -> Json<MCPServiceGenericResponse> {
        match self
            .mcp_implementation
            .read_blocks(MCPReadBlocksRequest {
                block_ids,
                has_payload,
            })
            .await
        {
            Ok(result) => {
                let value = serde_json::to_value(result).unwrap();
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
}

#[tool_handler]
impl ServerHandler for OpenNoteMCPService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(INSTRUCTIONS)
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

        Ok(self.get_info())
    }
}
