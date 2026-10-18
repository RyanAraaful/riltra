// SPDX-License-Identifier: Apache-2.0
//! M8 CP1: minimal MCP stdio server skeleton (design gate §3, CP1 scope).
//! Serves initialize and an empty `tools/list`; the 11 read-only tools arrive
//! in CP3+. Never opens repository storage. stdout stays protocol-only.

use rmcp::model::{
    Implementation, ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerConfig,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::ErrorData as McpError;
use rmcp::{serve_server, ServerHandler};

struct RiltraMcp;

impl ServerHandler for RiltraMcp {
    fn get_info(&self) -> ServerConfig {
        // Override the SDK default (`Implementation::from_build_env` reads
        // rmcp's own crate name) with this binary's identity.
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(crate::NAME, env!("CARGO_PKG_VERSION")))
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        // CP1 advertises no tools on purpose: never register placeholders.
        Ok(ListToolsResult::with_all_items(Vec::new()))
    }
}

/// Run the MCP stdio server to completion. Exit codes follow the CLI:
/// 0 clean shutdown (client EOF/disconnect), 3 runtime failure.
pub(crate) fn run() -> i32 {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("error: mcp runtime: {e}");
            return 3;
        }
    };
    runtime.block_on(async {
        let service = match serve_server(RiltraMcp, rmcp::transport::io::stdio()).await {
            Ok(service) => service,
            Err(e) => {
                eprintln!("error: mcp initialize failed: {e}");
                return 3;
            }
        };
        match service.waiting().await {
            Ok(rmcp::service::QuitReason::JoinError(e)) => {
                eprintln!("error: mcp task failed: {e}");
                3
            }
            Ok(_) => 0,
            Err(e) => {
                eprintln!("error: mcp task failed: {e}");
                3
            }
        }
    })
}
