//! Calculator service for testing MCP tool functionality.
//!
//! This module provides a simple calculator service that demonstrates
//! MCP tool implementation using the `tool_router` and `tool_handler` macros.
//! It's used in integration tests to verify that the transport layer correctly
//! handles tool calls and responses.

#![allow(dead_code)]
use rmcp::{
    ServerHandler,
    handler::server::{
        router::tool::ToolRouter,
        wrapper::{Json, Parameters},
    },
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
};

/// Request structure for the sum operation.
///
/// Demonstrates how to define typed parameters for MCP tools
/// with JSON schema generation support.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SumRequest {
    #[schemars(description = "the left hand side number")]
    pub a: i32,
    /// The right hand side number to add
    pub b: i32,
}

/// Request structure for the subtraction operation.
///
/// Similar to SumRequest but for subtraction, showing consistent
/// parameter definition patterns.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SubRequest {
    #[schemars(description = "the left hand side number")]
    pub a: i32,
    #[schemars(description = "the right hand side number")]
    pub b: i32,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct CalculatorResult {
    #[schemars(description = "the result of the operation")]
    pub value: i32,
}

impl From<i32> for CalculatorResult {
    fn from(value: i32) -> Self {
        Self { value }
    }
}

/// A simple calculator service for testing MCP tool functionality.
///
/// This service implements basic arithmetic operations (sum and sub)
/// as MCP tools. It demonstrates:
/// - Using the `tool_router` macro to generate tool routing
/// - Implementing typed tool methods with structured parameters
/// - Proper MCP service initialization with capabilities
///
/// # Example
///
/// ```no_run
/// let calculator = Calculator::new();
/// // The calculator can now handle MCP tool calls for 'sum' and 'sub'
/// ```
#[derive(Debug, Clone)]
pub struct Calculator {
    tool_router: ToolRouter<Self>,
}

impl Calculator {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

impl Default for Calculator {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl Calculator {
    #[tool(description = "Calculate the sum of two numbers")]
    fn sum(
        &self,
        Parameters(SumRequest { a, b }): Parameters<SumRequest>,
    ) -> Json<CalculatorResult> {
        Json((a + b).into())
    }

    #[tool(description = "Calculate the sub of two numbers")]
    fn sub(
        &self,
        Parameters(SubRequest { a, b }): Parameters<SubRequest>,
    ) -> Json<CalculatorResult> {
        Json((a - b).into())
    }
}

#[tool_handler]
impl ServerHandler for Calculator {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("A simple calculator")
    }
}
