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

        let mut server = Server::new("mcp-webcam", "0.1.0");

        // Create tool handlers as closures
        let list_cameras_handler = {
            let manager = Arc::clone(&self.webcam_manager);
            Box::new(move |_params: Value| -> Result<Value, MCPError> {
                debug!("Handling list_cameras request");
                
                let manager = manager.lock()
                    .map_err(|e| MCPError::Protocol(format!("Failed to acquire webcam manager lock: {}", e)))?;
                
                match manager.list_cameras() {
                    Ok(cameras) => {
                        info!("Found {} cameras", cameras.len());
                        Ok(json!({
                            "content": [{
                                "type": "text",
                                "text": format!("Found {} camera(s)", cameras.len())
                            }],
                            "cameras": cameras
                        }))
                    }
                    Err(e) => {
                        error!("Failed to list cameras: {}", e);
                        Ok(json!({
                            "content": [{
                                "type": "text", 
                                "text": format!("Error listing cameras: {}", e)
                            }],
                            "cameras": []
                        }))
                    }
                }
            })
        };

        let capture_image_handler = {
            let manager = Arc::clone(&self.webcam_manager);
            Box::new(move |params: Value| -> Result<Value, MCPError> {
                debug!("Handling capture_image request with params: {}", params);
                
                // Parse camera index from params (optional)
                let camera_index = params.get("camera_index")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);

                let mut manager = manager.lock()
                    .map_err(|e| MCPError::Protocol(format!("Failed to acquire webcam manager lock: {}", e)))?;

                match manager.capture_image(camera_index) {
                    Ok(result) => {
                        info!("Successfully captured image from camera {}", result.camera_index);
                        Ok(json!({
                            "content": [
                                {
                                    "type": "image",
                                    "data": result.image_data,
                                    "mimeType": result.mime_type
                                },
                                {
                                    "type": "text",
                                    "text": format!(
                                        "Captured {}x{} image from camera {} at {}",
                                        result.width, result.height, result.camera_index, result.timestamp
                                    )
                                }
                            ],
                            "metadata": {
                                "width": result.width,
                                "height": result.height,
                                "camera_index": result.camera_index,
                                "timestamp": result.timestamp,
                                "mime_type": result.mime_type
                            }
                        }))
                    }
                    Err(e) => {
                        error!("Failed to capture image: {}", e);
                        Ok(json!({
                            "content": [{
                                "type": "text",
                                "text": format!("Error capturing image: {}", e)
                            }],
                            "error": e.to_string()
                        }))
                    }
                }
            })
        };

        let get_camera_info_handler = {
            let manager = Arc::clone(&self.webcam_manager);
            Box::new(move |_params: Value| -> Result<Value, MCPError> {
                debug!("Handling get_camera_info request");
                
                let manager = manager.lock()
                    .map_err(|e| MCPError::Protocol(format!("Failed to acquire webcam manager lock: {}", e)))?;
                
                let current_camera = manager.get_current_camera_info();
                
                match manager.list_cameras() {
                    Ok(cameras) => {
                        let info = json!({
                            "available_cameras": cameras,
                            "current_camera": current_camera,
                            "total_cameras": cameras.len()
                        });

                        Ok(json!({
                            "content": [{
                                "type": "text",
                                "text": format!("Camera info: {} total cameras, current: {:?}", 
                                              cameras.len(), current_camera)
                            }],
                            "camera_info": info
                        }))
                    }
                    Err(e) => {
                        error!("Failed to get camera info: {}", e);
                        Ok(json!({
                            "content": [{
                                "type": "text",
                                "text": format!("Error getting camera info: {}", e)
                            }],
                            "error": e.to_string()
                        }))
                    }
                }
            })
        };

        // Add local camera tools
        server.add_tool("list_cameras", "List all available local camera devices", list_cameras_handler);
        server.add_tool("capture_image", "Capture an image from a local camera", capture_image_handler);
        server.add_tool("get_camera_info", "Get information about available local cameras", get_camera_info_handler);

        // Add Shodan tools if API key is available
        if let Some(shodan_client) = &self.shodan_client {
            let search_webcams_handler = {
                let client = shodan_client.clone();
                Box::new(move |params: Value| -> Result<Value, MCPError> {
                    // This needs to be async, but the trait signature is sync
                    // For now, return a placeholder
                    Ok(json!({
                        "content": [{
                            "type": "text",
                            "text": "Shodan webcam search not yet implemented in sync context"
                        }],
                        "note": "This feature requires async support"
                    }))
                })
            };

            server.add_tool("search_webcams", "Search for remote webcams using Shodan", search_webcams_handler);
        }

        info!("MCP server configured, starting stdio transport");
        server.run_stdio().await?;
        
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
