mod webcam;
mod mcp_server;
mod shodan;

use crate::mcp_server::WebcamMcpServer;
use anyhow::Result;
use tracing::{error, info};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .init();

    info!("MCP Webcam Server starting...");

    // Create and run the MCP server
    let server = WebcamMcpServer::new();
    
    match server.run().await {
        Ok(_) => {
            info!("MCP Webcam Server stopped gracefully");
            Ok(())
        }
        Err(e) => {
            error!("MCP Webcam Server error: {}", e);
            Err(e)
        }
    }
}
