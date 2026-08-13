//! Common test utilities for rmcp-actix-web integration tests.
//!
//! This module provides shared test services and utilities used across
//! different integration tests, including JavaScript and Python client tests.

/// Calculator service implementation for testing MCP tool functionality.
pub mod calculator;

/// Test service for verifying Authorization header forwarding.
pub mod headers_test_service;

/// Helpers for reading JSON payloads out of SSE responses.
pub mod sse;
