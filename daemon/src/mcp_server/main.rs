// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// convergio-mcp-server binary entry point.
// Exposes the Convergio daemon as an MCP server over stdio transport.
// CRITICAL: stdout is JSON-RPC protocol only. All logs must go to stderr.

use convergio_core::mcp_server::McpServer;

fn main() {
    // Route tracing output to stderr — stdout is reserved for JSON-RPC.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    // Ring level from environment: 0=Core, 1=Trusted, 2=Community, 3=Sandboxed (default).
    let ring: u8 = std::env::var("CONVERGIO_MCP_RING")
        .ok() // intentional: missing env falls back to sandbox ring for safe startup
        .and_then(|v| v.parse().ok()) // intentional: invalid ring string falls back to sandbox default
        .unwrap_or(3);

    // Daemon URL — defaults to local daemon.
    let daemon_url = std::env::var("CONVERGIO_DAEMON_URL")
        .unwrap_or_else(|_| "http://localhost:8420".to_string());

    // Optional bearer token for daemon auth middleware.
    let token = std::env::var("CONVERGIO_API_TOKEN").ok(); // intentional: token is optional in local/dev deployments

    tracing::info!(
        ring,
        daemon_url = %daemon_url,
        "convergio-mcp-server starting"
    );

    let server = McpServer::new(ring, &daemon_url, token.as_deref());
    server.run_stdio();
}
