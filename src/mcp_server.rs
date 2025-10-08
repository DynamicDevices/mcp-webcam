use crate::webcam::WebcamManager;
use crate::shodan::{ShodanClient, RemoteWebcam, WebcamAccessType};
use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};

// Import mcpr types
use mcpr::server::{Server, ServerConfig};
use mcpr::schema::common::{Tool, ToolInputSchema};
use mcpr::transport::stdio::StdioTransport;
use mcpr::error::MCPError;

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
        info!("🚀 Starting MCP Webcam Server with full protocol support");
        
        // Create server configuration with tools
        let mut config = ServerConfig::new()
            .with_name("mcp-webcam")
            .with_version("0.1.0");

        // Add local camera tools
        config = config.with_tool(Tool {
            name: "list_cameras".to_string(),
            description: Some("List all available local camera devices".to_string()),
            input_schema: ToolInputSchema {
                r#type: "object".to_string(),
                properties: Some(std::collections::HashMap::new()),
                required: None,
            },
        });

        config = config.with_tool(Tool {
            name: "capture_image".to_string(),
            description: Some("Capture an image from a local camera".to_string()),
            input_schema: ToolInputSchema {
                r#type: "object".to_string(),
                properties: Some({
                    let mut props = std::collections::HashMap::new();
                    props.insert("camera_index".to_string(), json!({
                        "type": "number",
                        "description": "Camera index to use (optional, defaults to 0)"
                    }));
                    props
                }),
                required: None,
            },
        });

        config = config.with_tool(Tool {
            name: "get_camera_info".to_string(),
            description: Some("Get information about available local cameras".to_string()),
            input_schema: ToolInputSchema {
                r#type: "object".to_string(),
                properties: Some(std::collections::HashMap::new()),
                required: None,
            },
        });

        // Add Shodan tools if API key is available
        if self.shodan_client.is_some() {
            config = config.with_tool(Tool {
                name: "search_webcams".to_string(),
                description: Some("Search for remote webcams using Shodan".to_string()),
                input_schema: ToolInputSchema {
                    r#type: "object".to_string(),
                    properties: Some({
                        let mut props = std::collections::HashMap::new();
                        props.insert("limit".to_string(), json!({
                            "type": "number",
                            "description": "Maximum number of results (optional, defaults to 20)"
                        }));
                        props
                    }),
                    required: None,
                },
            });

            config = config.with_tool(Tool {
                name: "capture_remote_image".to_string(),
                description: Some("Capture image from a remote webcam".to_string()),
                input_schema: ToolInputSchema {
                    r#type: "object".to_string(),
                    properties: Some({
                        let mut props = std::collections::HashMap::new();
                        props.insert("url".to_string(), json!({
                            "type": "string",
                            "description": "Webcam URL to capture from"
                        }));
                        props.insert("ip".to_string(), json!({
                            "type": "string",
                            "description": "IP address (optional)"
                        }));
                        props.insert("port".to_string(), json!({
                            "type": "number",
                            "description": "Port number (optional)"
                        }));
                        props
                    }),
                    required: Some(vec!["url".to_string()]),
                },
            });
        }

        // Create server and register tool handlers
        let mut server: Server<StdioTransport> = Server::new(config);
        
        // Register local camera tool handlers
        self.register_local_camera_tools(&mut server)?;
        
        // Register Shodan tool handlers if available
        if self.shodan_client.is_some() {
            self.register_shodan_tools(&mut server)?;
        }

        info!("📷 Local camera tools registered: list_cameras, capture_image, get_camera_info");
        if self.shodan_client.is_some() {
            info!("🌐 Shodan tools registered: search_webcams, capture_remote_image");
        }
        info!("✅ MCP Webcam Server ready - starting stdio transport");

        // Create stdio transport and start server
        let transport = StdioTransport::new();
        server.start(transport)?;

        Ok(())
    }

    fn register_local_camera_tools(&self, server: &mut Server<StdioTransport>) -> Result<(), MCPError> {
        // Clone references for closures
        let webcam_manager_list = Arc::clone(&self.webcam_manager);
        let webcam_manager_capture = Arc::clone(&self.webcam_manager);
        let webcam_manager_info = Arc::clone(&self.webcam_manager);

        // Register list_cameras handler
        server.register_tool_handler("list_cameras", move |_params: Value| -> Result<Value, MCPError> {
            debug!("Handling list_cameras request");
            
            let manager = webcam_manager_list.lock()
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
        })?;

        // Register capture_image handler
        server.register_tool_handler("capture_image", move |params: Value| -> Result<Value, MCPError> {
            debug!("Handling capture_image request with params: {}", params);
            
            // Parse camera index from params (optional)
            let camera_index = params.get("camera_index")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32);

            let mut manager = webcam_manager_capture.lock()
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
        })?;

        // Register get_camera_info handler
        server.register_tool_handler("get_camera_info", move |_params: Value| -> Result<Value, MCPError> {
            debug!("Handling get_camera_info request");
            
            let manager = webcam_manager_info.lock()
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
        })?;

        Ok(())
    }

    fn register_shodan_tools(&self, server: &mut Server<StdioTransport>) -> Result<(), MCPError> {
        let shodan_client_search = self.shodan_client.clone().unwrap();
        let shodan_client_capture = self.shodan_client.clone().unwrap();

        // Register search_webcams handler
        server.register_tool_handler("search_webcams", move |params: Value| -> Result<Value, MCPError> {
            debug!("Handling search_webcams request with params: {}", params);
            
            // Parse limit from params (optional)
            let limit = params.get("limit")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32);

            // Create a runtime for async execution
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| MCPError::Protocol(format!("Failed to create async runtime: {}", e)))?;

            match rt.block_on(shodan_client_search.search_webcams(limit)) {
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
        })?;

        // Register capture_remote_image handler
        server.register_tool_handler("capture_remote_image", move |params: Value| -> Result<Value, MCPError> {
            debug!("Handling capture_remote_image request with params: {}", params);
            
            // Parse webcam URL from params
            let webcam_url = params.get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| MCPError::Protocol("Missing required parameter 'url'".to_string()))?;

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

            // Create a runtime for async execution
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| MCPError::Protocol(format!("Failed to create async runtime: {}", e)))?;

            match rt.block_on(shodan_client_capture.fetch_webcam_image(&webcam)) {
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
        })?;

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
