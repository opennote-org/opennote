use rmcp::handler::server::tool::ToolRouter;

use crate::mcp::search::Search;

#[derive(Debug, Clone)]
pub struct MCPService {
    search: ToolRouter<Search>
}