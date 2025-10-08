use crate::webcam::WebcamManager;
use crate::shodan::ShodanClient;
use anyhow::Result;
use serde_json::json;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

// Import mcpr types from their correct modules
use mcpr::server::{Server, ServerConfig};
use mcpr::schema::common::Tool;

pub struct WebcamMcpServer {
    webcam_manager: Arc<Mutex<WebcamManager>>,
    shodan_client: Option<ShodanClient>,
}

impl WebcamMcpServer {
    pub fn new() -> Self {
        let shodan_client = std::env::var("SHODAN_API_KEY")
            .ok()
            .map(ShodanClient::new);
            
        if shodan_client.is_some() {
            info!("Shodan integration enabled");
        } else {
            warn!("SHODAN_API_KEY not found - Shodan features will be disabled");
        }

        Self {
            webcam_manager: Arc::new(Mutex::new(WebcamManager::new())),
            shodan_client,
        }
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting MCP Webcam Server");

        // Create server configuration
        let mut config = ServerConfig::new()
            .with_name("mcp-webcam")
            .with_version("0.1.0");

        // Create tools and add them to config
        let list_cameras_tool = Tool {
            name: "list_cameras".to_string(),
            description: Some("List all available local camera devices".to_string()),
            input_schema: json!({"type": "object", "properties": {}}),
        };
        config = config.with_tool(list_cameras_tool);

        let capture_image_tool = Tool {
            name: "capture_image".to_string(),
            description: Some("Capture an image from a local camera".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "camera_index": {"type": "number", "description": "Camera index to use (optional)"}
                }
            }),
        };
        config = config.with_tool(capture_image_tool);

        let get_camera_info_tool = Tool {
            name: "get_camera_info".to_string(),
            description: Some("Get information about available local cameras".to_string()),
            input_schema: json!({"type": "object", "properties": {}}),
        };
        config = config.with_tool(get_camera_info_tool);

        // Add Shodan tools if API key is available
        if self.shodan_client.is_some() {
            let search_webcams_tool = Tool {
                name: "search_webcams".to_string(),
                description: Some("Search for remote webcams using Shodan".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "limit": {"type": "number", "description": "Maximum number of results (optional)"}
                    }
                }),
            };
            config = config.with_tool(search_webcams_tool);
        }

        let _server = Server::new(config);

        info!("MCP server configured");
        info!("🚀 MCP Webcam Server is ready!");
        info!("📷 Local camera tools: list_cameras, capture_image, get_camera_info");
        if self.shodan_client.is_some() {
            info!("🌐 Shodan tools: search_webcams");
        }
        info!("⚠️  Note: Full MCP protocol implementation is still in progress");
        info!("📖 See README.md for usage instructions and safety guidelines");
        
        // For now, just keep the server alive
        // In a full implementation, this would handle MCP protocol over stdio
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        
        Ok(())
    }
}

impl Clone for WebcamMcpServer {
    fn clone(&self) -> Self {
        Self {
            webcam_manager: Arc::clone(&self.webcam_manager),
            shodan_client: self.shodan_client.clone(),
        }
    }
}
