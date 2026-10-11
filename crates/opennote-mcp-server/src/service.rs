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
    requests::{
        MCPCreateBlockRequest, MCPDeleteBlocksRequest, MCPReadBlockDetailsRequest,
        MCPReadBlocksRequest, MCPSearchRequest, MCPUpdateBlocksRequest,
    },
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
                tracing::warn!("MCP service reported error: {}", error);
                return Json(MCPServiceGenericResponse { results: None });
            }
        }
    }

    #[tool(description = "read user's OpenNote blocks")]
    pub async fn read_blocks(
        &self,
        Parameters(MCPReadBlocksRequest { block_ids }): Parameters<MCPReadBlocksRequest>,
    ) -> Json<MCPServiceGenericResponse> {
        match self
            .mcp_implementation
            .read_blocks(MCPReadBlocksRequest { block_ids })
            .await
        {
            Ok(result) => {
                let value = serde_json::to_value(result).unwrap();
                return Json(MCPServiceGenericResponse {
                    results: Some(value),
                });
            }
            Err(error) => {
                tracing::warn!("MCP service reported error: {}", error);
                return Json(MCPServiceGenericResponse { results: None });
            }
        }
    }

    #[tool(
        description = "read the full text content of an OpenNote block. read blocks to get the block_id first"
    )]
    pub async fn read_block_details(
        &self,
        Parameters(request): Parameters<MCPReadBlockDetailsRequest>,
    ) -> Json<MCPServiceGenericResponse> {
        match self.mcp_implementation.read_block_details(request).await {
            Ok(result) => {
                return Json(MCPServiceGenericResponse {
                    results: Some(serde_json::Value::String(result)),
                });
            }
            Err(error) => {
                tracing::warn!("MCP service reported error: {}", error);
                return Json(MCPServiceGenericResponse { results: None });
            }
        }
    }

    #[tool(
        description = "update an OpenNote block by inserting text at the start or end, then optionally replacing the first regex match. read blocks to get the block_id first. returns true on success"
    )]
    pub async fn update_block(
        &self,
        Parameters(request): Parameters<MCPUpdateBlocksRequest>,
    ) -> Json<MCPServiceGenericResponse> {
        match self.mcp_implementation.update_block(request).await {
            Ok(()) => {
                return Json(MCPServiceGenericResponse {
                    results: Some(serde_json::Value::Bool(true)),
                });
            }
            Err(error) => {
                tracing::warn!("MCP service reported error: {}", error);
                return Json(MCPServiceGenericResponse { results: None });
            }
        }
    }

    #[tool(
        description = "create an OpenNote block with a title in the specified server scope (`local` for local storage), optionally under a parent block. returns true on success"
    )]
    pub async fn create_block(
        &self,
        Parameters(request): Parameters<MCPCreateBlockRequest>,
    ) -> Json<MCPServiceGenericResponse> {
        match self.mcp_implementation.create_block(request).await {
            Ok(()) => {
                return Json(MCPServiceGenericResponse {
                    results: Some(serde_json::Value::Bool(true)),
                });
            }
            Err(error) => {
                tracing::warn!("MCP service reported error: {}", error);
                return Json(MCPServiceGenericResponse { results: None });
            }
        }
    }

    #[tool(
        description = "delete OpenNote blocks by their IDs in the specified server scope (`local` for local storage). read blocks to get block_ids first. returns true on success"
    )]
    pub async fn delete_blocks(
        &self,
        Parameters(request): Parameters<MCPDeleteBlocksRequest>,
    ) -> Json<MCPServiceGenericResponse> {
        match self.mcp_implementation.delete_blocks(request).await {
            Ok(()) => {
                return Json(MCPServiceGenericResponse {
                    results: Some(serde_json::Value::Bool(true)),
                });
            }
            Err(error) => {
                tracing::warn!("MCP service reported error: {}", error);
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
