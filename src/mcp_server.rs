use crate::webcam::WebcamManager;
use crate::shodan::ShodanClient;
use anyhow::Result;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

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
        info!("🚀 MCP Webcam Server is starting...");
        
        // Display available features
        info!("📷 Local camera features:");
        info!("  - list_cameras: List available local cameras");
        info!("  - capture_image: Capture image from local camera");
        info!("  - get_camera_info: Get camera information");
        
        if self.shodan_client.is_some() {
            info!("🌐 Shodan integration features:");
            info!("  - search_webcams: Search for remote webcams");
            info!("  - capture_remote_image: Capture from remote webcam");
        } else {
            info!("⚠️  Shodan features disabled (no API key)");
        }
        
        info!("📖 See README.md for detailed usage instructions");
        info!("⚠️  IMPORTANT: Only access webcams you own or have permission to use");
        
        // Test local camera functionality if enabled
        #[cfg(feature = "local_cameras")]
        {
            let manager = self.webcam_manager.lock().unwrap();
            match manager.list_cameras() {
                Ok(cameras) => {
                    info!("✅ Found {} local camera(s)", cameras.len());
                }
                Err(e) => {
                    warn!("⚠️  Local camera access failed: {}", e);
                }
            }
        }
        
        #[cfg(not(feature = "local_cameras"))]
        {
            info!("ℹ️  Local camera support not compiled in");
        }
        
        info!("✅ MCP Webcam Server is ready!");
        info!("🔧 Note: Full MCP protocol implementation is in development");
        
        // Keep server running
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
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
