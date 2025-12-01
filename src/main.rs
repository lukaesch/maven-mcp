use anyhow::Result;
use rmcp::transport::stdio;
use rmcp::ServiceExt;
use tracing::info;
use tracing_subscriber::{self, EnvFilter};

use maven_mcp::MavenToolsService;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging (to stderr, not stdout which is used for MCP communication)
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("maven_mcp=info".parse()?))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    info!("Starting maven-mcp v{}", env!("CARGO_PKG_VERSION"));

    // Create the service
    let service = MavenToolsService::new();

    // Start the server with stdio transport
    let server = service.serve(stdio()).await?;

    info!("Server initialized, waiting for requests...");

    // Wait for the server to complete
    let _result = server.waiting().await?;

    info!("Server shutting down");

    Ok(())
}
