use crate::webcam::{WebcamManager, WebcamError, CameraInfo, CaptureResult};
use crate::shodan::{ShodanClient, ShodanError, RemoteWebcam, WebcamAccessType};
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use mcpr::prelude::*;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};

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

        let mut server_builder = McpServer::new("mcp-webcam", "0.1.0")
            .with_tool("list_cameras", "List all available local camera devices", self.clone())
            .with_tool("capture_image", "Capture an image from a local camera", self.clone())
            .with_tool("get_camera_info", "Get information about available local cameras", self.clone());

        // Add Shodan tools if API key is available
        if self.shodan_client.is_some() {
            server_builder = server_builder
                .with_tool("search_webcams", "Search for remote webcams using Shodan", self.clone())
                .with_tool("capture_remote_image", "Capture image from a remote webcam", self.clone())
                .with_tool("list_remote_webcams", "List discovered remote webcams", self.clone());
        }

        let server = server_builder.build();

        info!("MCP server configured, starting stdio transport");
        server.run_stdio().await?;
        
        Ok(())
    }

    async fn handle_list_cameras(&self, _params: Value) -> Result<Value> {
        debug!("Handling list_cameras request");
        
        let manager = self.webcam_manager.lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire webcam manager lock: {}", e))?;
        
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
    }

    async fn handle_capture_image(&self, params: Value) -> Result<Value> {
        debug!("Handling capture_image request with params: {}", params);
        
        // Parse camera index from params (optional)
        let camera_index = params.get("camera_index")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        let mut manager = self.webcam_manager.lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire webcam manager lock: {}", e))?;

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
    }

    async fn handle_get_camera_info(&self, _params: Value) -> Result<Value> {
        debug!("Handling get_camera_info request");
        
        let manager = self.webcam_manager.lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire webcam manager lock: {}", e))?;
        
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

#[async_trait::async_trait]
impl ToolHandler for WebcamMcpServer {
    async fn handle_tool(&self, name: &str, params: Value) -> Result<Value> {
        match name {
            "list_cameras" => self.handle_list_cameras(params).await,
            "capture_image" => self.handle_capture_image(params).await,
            "get_camera_info" => self.handle_get_camera_info(params).await,
            "search_webcams" => self.handle_search_webcams(params).await,
            "capture_remote_image" => self.handle_capture_remote_image(params).await,
            "list_remote_webcams" => self.handle_list_remote_webcams(params).await,
            _ => {
                warn!("Unknown tool requested: {}", name);
                Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Unknown tool: {}", name)
                    }],
                    "error": "Tool not found"
                }))
            }
        }
    }
}
