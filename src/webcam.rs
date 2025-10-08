use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{error, warn};

#[cfg(feature = "local_cameras")]
use {
    anyhow::Context,
    base64::{engine::general_purpose, Engine as _},
    image::{ImageFormat},
    std::io::Cursor,
    tracing::{debug, info},
    nokhwa::{
        pixel_format::RgbFormat,
        utils::{CameraIndex, RequestedFormat, RequestedFormatType, Resolution},
        Camera, CallbackCamera,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraInfo {
    pub index: u32,
    pub name: String,
    pub description: String,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureResult {
    pub image_data: String, // Base64 encoded
    pub mime_type: String,
    pub width: u32,
    pub height: u32,
    pub timestamp: String,
    pub camera_index: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum WebcamError {
    #[error("Camera not found: {index}")]
    CameraNotFound { index: u32 },
    #[cfg(feature = "local_cameras")]
    #[error("Failed to open camera: {0}")]
    CameraOpen(#[from] nokhwa::NokhwaError),
    #[error("Image processing error: {0}")]
    ImageProcessing(#[from] image::ImageError),
    #[error("No cameras available")]
    NoCamerasAvailable,
    #[error("Local camera support not compiled in")]
    LocalCamerasNotSupported,
}

pub struct WebcamManager {
    #[cfg(feature = "local_cameras")]
    current_camera: Option<Camera>,
    #[cfg(not(feature = "local_cameras"))]
    current_camera: Option<()>, // Placeholder when local cameras not supported
    current_index: Option<u32>,
}

impl WebcamManager {
    pub fn new() -> Self {
        Self {
            current_camera: None,
            current_index: None,
        }
    }

    /// List all available cameras
    pub fn list_cameras(&self) -> Result<Vec<CameraInfo>, WebcamError> {
        #[cfg(feature = "local_cameras")]
        {
            info!("Listing available cameras");
            
            match nokhwa::query_devices() {
                Ok(devices) => {
                    let cameras: Vec<CameraInfo> = devices
                        .into_iter()
                        .enumerate()
                        .map(|(index, device)| CameraInfo {
                            index: index as u32,
                            name: device.human_name().to_string(),
                            description: device.description().to_string(),
                            available: true,
                        })
                        .collect();

                    info!("Found {} cameras", cameras.len());
                    Ok(cameras)
                }
                Err(e) => {
                    warn!("Failed to query devices: {}", e);
                    Ok(vec![]) // Return empty list instead of error
                }
            }
        }
        
        #[cfg(not(feature = "local_cameras"))]
        {
            warn!("Local camera support not compiled in");
            Err(WebcamError::LocalCamerasNotSupported)
        }
    }

    /// Open a specific camera by index
    pub fn open_camera(&mut self, _index: u32) -> Result<(), WebcamError> {
        #[cfg(feature = "local_cameras")]
        {
            info!("Opening camera {}", index);

            // Close current camera if open
            if self.current_camera.is_some() {
                debug!("Closing current camera");
                self.current_camera = None;
                self.current_index = None;
            }

            // Try to open the requested camera
            let camera_index = CameraIndex::Index(index);
            let requested_format = RequestedFormat::new::<RgbFormat>(
                RequestedFormatType::AbsoluteHighestResolution,
            );

            match Camera::new(camera_index, requested_format) {
                Ok(camera) => {
                    info!("Successfully opened camera {}", index);
                    self.current_camera = Some(camera);
                    self.current_index = Some(index);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to open camera {}: {}", index, e);
                    Err(WebcamError::CameraOpen(e))
                }
            }
        }
        
        #[cfg(not(feature = "local_cameras"))]
        {
            error!("Local camera support not compiled in");
            Err(WebcamError::LocalCamerasNotSupported)
        }
    }

    /// Capture an image from the current or specified camera
    pub fn capture_image(&mut self, _camera_index: Option<u32>) -> Result<CaptureResult, WebcamError> {
        #[cfg(feature = "local_cameras")]
        {
            let target_index = camera_index.unwrap_or(0);

            // Open camera if not already open or if different camera requested
            if self.current_camera.is_none() || self.current_index != Some(target_index) {
                self.open_camera(target_index)?;
            }

            let camera = self.current_camera.as_mut()
                .ok_or(WebcamError::CameraNotFound { index: target_index })?;

            info!("Capturing frame from camera {}", target_index);

            // Capture frame
            let frame = camera.frame()
                .context("Failed to capture frame")
                .map_err(|e| WebcamError::CameraOpen(nokhwa::NokhwaError::GeneralError(e.to_string())))?;

            let width = frame.width();
            let height = frame.height();

            debug!("Captured frame: {}x{}", width, height);

            // Convert to RGB image
            let rgb_data = frame.into_rgb8();
            let img = image::ImageBuffer::from_raw(width, height, rgb_data.into_raw())
                .ok_or_else(|| WebcamError::ImageProcessing(image::ImageError::Parameter(
                    image::error::ParameterError::from_kind(image::error::ParameterErrorKind::DimensionMismatch)
                )))?;

            // Encode as JPEG
            let mut buffer = Cursor::new(Vec::new());
            img.write_to(&mut buffer, ImageFormat::Jpeg)?;
            let image_bytes = buffer.into_inner();

            // Encode as base64
            let image_data = general_purpose::STANDARD.encode(&image_bytes);

            let result = CaptureResult {
                image_data,
                mime_type: "image/jpeg".to_string(),
                width,
                height,
                timestamp: chrono::Utc::now().to_rfc3339(),
                camera_index: target_index,
            };

            info!("Successfully captured image: {}x{} from camera {}", width, height, target_index);
            Ok(result)
        }
        
        #[cfg(not(feature = "local_cameras"))]
        {
            error!("Local camera support not compiled in");
            Err(WebcamError::LocalCamerasNotSupported)
        }
    }

    /// Get information about the currently open camera
    pub fn get_current_camera_info(&self) -> Option<u32> {
        self.current_index
    }
}

impl Default for WebcamManager {
    fn default() -> Self {
        Self::new()
    }
}

// Add chrono dependency for timestamps
