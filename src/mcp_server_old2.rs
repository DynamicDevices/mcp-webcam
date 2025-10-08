use crate::webcam::WebcamManager;
use crate::shodan::{ShodanClient, RemoteWebcam, WebcamAccessType};
use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};

// Import mcpr types from their correct modules
use mcpr::server::{Server, ToolHandler};

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

        // Add local camera tools
        server.add_tool("list_cameras", "List all available local camera devices", Box::new(self.clone()));
        server.add_tool("capture_image", "Capture an image from a local camera", Box::new(self.clone()));
        server.add_tool("get_camera_info", "Get information about available local cameras", Box::new(self.clone()));

        // Add Shodan tools if API key is available
        if self.shodan_client.is_some() {
            server.add_tool("search_webcams", "Search for remote webcams using Shodan", Box::new(self.clone()));
            server.add_tool("capture_remote_image", "Capture image from a remote webcam", Box::new(self.clone()));
            server.add_tool("list_remote_webcams", "List discovered remote webcams", Box::new(self.clone()));
        }

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

    async fn handle_search_webcams(&self, params: Value) -> Result<Value> {
        debug!("Handling search_webcams request with params: {}", params);
        
        let shodan_client = match &self.shodan_client {
            Some(client) => client,
            None => {
                return Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": "Shodan integration not available. Please set SHODAN_API_KEY environment variable."
                    }],
                    "error": "No Shodan API key"
                }));
            }
        };

        // Parse limit from params (optional)
        let limit = params.get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .or(Some(20)); // Default limit

        match shodan_client.search_webcams(limit).await {
            Ok(webcams) => {
                info!("Found {} remote webcams via Shodan", webcams.len());
                Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Found {} remote webcam(s) via Shodan search", webcams.len())
                    }],
                    "webcams": webcams,
                    "total": webcams.len()
                }))
            }
            Err(e) => {
                error!("Failed to search webcams via Shodan: {}", e);
                Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Error searching webcams via Shodan: {}", e)
                    }],
                    "error": e.to_string()
                }))
            }
        }
    }

    async fn handle_capture_remote_image(&self, params: Value) -> Result<Value> {
        debug!("Handling capture_remote_image request with params: {}", params);
        
        let shodan_client = match &self.shodan_client {
            Some(client) => client,
            None => {
                return Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": "Shodan integration not available. Please set SHODAN_API_KEY environment variable."
                    }],
                    "error": "No Shodan API key"
                }));
            }
        };

        // Parse webcam URL from params
        let webcam_url = params.get("url")
            .and_then(|v| v.as_str());

        let webcam_url = match webcam_url {
            Some(url) => url,
            None => {
                return Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": "Please provide 'url' parameter with the webcam URL"
                    }],
                    "error": "Missing required parameters"
                }));
            }
        };

        // Create a temporary RemoteWebcam struct for the fetch operation
        let webcam = RemoteWebcam {
            ip: params.get("ip").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
            port: params.get("port").and_then(|v| v.as_u64()).unwrap_or(80) as u16,
            url: webcam_url.to_string(),
            hostname: None,
            location: None,
            org: None,
            product: None,
            last_seen: chrono::Utc::now().to_rfc3339(),
            access_type: WebcamAccessType::HTTP,
        };

        match shodan_client.fetch_webcam_image(&webcam).await {
            Ok(image_bytes) => {
                let image_data = general_purpose::STANDARD.encode(&image_bytes);
                info!("Successfully captured remote image from {}", webcam_url);
                
                Ok(json!({
                    "content": [
                        {
                            "type": "image",
                            "data": image_data,
                            "mimeType": "image/jpeg"
                        },
                        {
                            "type": "text",
                            "text": format!("Captured image from remote webcam: {}", webcam_url)
                        }
                    ],
                    "metadata": {
                        "source": "remote_webcam",
                        "url": webcam_url,
                        "size_bytes": image_bytes.len(),
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    }
                }))
            }
            Err(e) => {
                error!("Failed to capture remote image from {}: {}", webcam_url, e);
                Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Error capturing remote image from {}: {}", webcam_url, e)
                    }],
                    "error": e.to_string()
                }))
            }
        }
    }

    async fn handle_list_remote_webcams(&self, _params: Value) -> Result<Value> {
        debug!("Handling list_remote_webcams request");
        
        match &self.shodan_client {
            Some(_client) => {
                Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": "Use the 'search_webcams' tool to discover remote webcams. This tool would typically show cached results from previous searches."
                    }],
                    "note": "This is a placeholder - implement webcam caching for full functionality"
                }))
            }
            None => {
                Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": "Shodan integration not available. Please set SHODAN_API_KEY environment variable."
                    }],
                    "error": "No Shodan API key"
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
