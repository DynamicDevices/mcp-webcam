use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShodanResult {
    pub ip: String,
    pub port: u16,
    pub hostname: Option<String>,
    pub location: Option<ShodanLocation>,
    pub org: Option<String>,
    pub data: String,
    pub timestamp: String,
    pub transport: String,
    pub product: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShodanLocation {
    pub country_name: Option<String>,
    pub city: Option<String>,
    pub region_code: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShodanSearchResponse {
    pub matches: Vec<ShodanResult>,
    pub total: u64,
    pub facets: Option<HashMap<String, Vec<ShodanFacet>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShodanFacet {
    pub count: u64,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteWebcam {
    pub ip: String,
    pub port: u16,
    pub url: String,
    pub hostname: Option<String>,
    pub location: Option<ShodanLocation>,
    pub org: Option<String>,
    pub product: Option<String>,
    pub last_seen: String,
    pub access_type: WebcamAccessType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebcamAccessType {
    MJPEG,
    RTSP,
    HTTP,
    Unknown,
}

#[derive(Debug, thiserror::Error)]
pub enum ShodanError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("API key not provided")]
    NoApiKey,
    #[error("Invalid query: {0}")]
    InvalidQuery(String),
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Unauthorized - check API key")]
    Unauthorized,
    #[error("Generic error: {0}")]
    Generic(String),
}

#[derive(Debug, Clone)]
pub struct ShodanClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl ShodanClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: "https://api.shodan.io".to_string(),
        }
    }

    /// Search for webcams using various common queries
    pub async fn search_webcams(&self, limit: Option<u32>) -> Result<Vec<RemoteWebcam>, ShodanError> {
        info!("Searching for webcams via Shodan");

        // Common webcam search queries
        let queries = vec![
            "Server: SQ-WEBCAM",
            "Server: yawcam",
            "Server: webcamXP",
            "\"Server: IP Webcam Server\"",
            "\"200 OK\" \"Content-Type: multipart/x-mixed-replace\"",
            "port:8080 \"mjpeg\"",
            "port:8081 \"mjpeg\"",
            "port:554 \"rtsp\"",
            "\"axis video server\"",
            "\"live view axis\"",
            "inurl:\"view/view.shtml\"",
            "inurl:\"ViewerFrame?Mode=\"",
            "inurl:\"MultiCameraFrame?Mode=\"",
        ];

        let mut all_webcams = Vec::new();
        let limit_per_query = limit.map(|l| l / queries.len() as u32).unwrap_or(10);

        for query in queries.iter().take(3) { // Limit to first 3 queries to avoid rate limits
            match self.search(query, Some(limit_per_query)).await {
                Ok(results) => {
                    let webcams = self.process_search_results(results);
                    all_webcams.extend(webcams);
                    
                    // Add small delay to avoid rate limiting
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
                Err(e) => {
                    warn!("Failed to search with query '{}': {}", query, e);
                }
            }
        }

        // Remove duplicates based on IP
        all_webcams.sort_by(|a, b| a.ip.cmp(&b.ip));
        all_webcams.dedup_by(|a, b| a.ip == b.ip);

        info!("Found {} unique webcams", all_webcams.len());
        Ok(all_webcams)
    }

    /// Generic search function
    pub async fn search(&self, query: &str, limit: Option<u32>) -> Result<ShodanSearchResponse, ShodanError> {
        debug!("Executing Shodan search: {}", query);

        let url = format!("{}/shodan/host/search", self.base_url);
        let mut params = vec![
            ("key", self.api_key.as_str()),
            ("query", query),
        ];

        let limit_str;
        if let Some(limit) = limit {
            limit_str = limit.to_string();
            params.push(("limit", &limit_str));
        }

        let response = self.client
            .get(&url)
            .query(&params)
            .send()
            .await?;

        match response.status() {
            reqwest::StatusCode::OK => {
                let search_response: ShodanSearchResponse = response.json().await?;
                debug!("Search returned {} results", search_response.matches.len());
                Ok(search_response)
            }
            reqwest::StatusCode::UNAUTHORIZED => Err(ShodanError::Unauthorized),
            reqwest::StatusCode::TOO_MANY_REQUESTS => Err(ShodanError::RateLimitExceeded),
            status => {
                let error_text = response.text().await.unwrap_or_default();
                error!("Shodan API error {}: {}", status, error_text);
                Err(ShodanError::Generic(format!("HTTP {}: {}", status, error_text)))
            }
        }
    }

    /// Process search results and extract webcam information
    fn process_search_results(&self, response: ShodanSearchResponse) -> Vec<RemoteWebcam> {
        response.matches
            .into_iter()
            .filter_map(|result| self.extract_webcam_info(result))
            .collect()
    }

    /// Extract webcam information from a Shodan result
    fn extract_webcam_info(&self, result: ShodanResult) -> Option<RemoteWebcam> {
        let access_type = self.determine_access_type(&result);
        let url = self.construct_webcam_url(&result, &access_type)?;

        Some(RemoteWebcam {
            ip: result.ip,
            port: result.port,
            url,
            hostname: result.hostname,
            location: result.location,
            org: result.org,
            product: result.product,
            last_seen: result.timestamp,
            access_type,
        })
    }

    /// Determine the type of webcam access based on the result data
    fn determine_access_type(&self, result: &ShodanResult) -> WebcamAccessType {
        let data_lower = result.data.to_lowercase();
        
        if data_lower.contains("mjpeg") || data_lower.contains("multipart/x-mixed-replace") {
            WebcamAccessType::MJPEG
        } else if result.port == 554 || data_lower.contains("rtsp") {
            WebcamAccessType::RTSP
        } else if result.port == 80 || result.port == 8080 || result.port == 8081 {
            WebcamAccessType::HTTP
        } else {
            WebcamAccessType::Unknown
        }
    }

    /// Construct a webcam URL based on the result and access type
    fn construct_webcam_url(&self, result: &ShodanResult, access_type: &WebcamAccessType) -> Option<String> {
        match access_type {
            WebcamAccessType::MJPEG => {
                // Common MJPEG endpoints
                let endpoints = vec![
                    "/mjpeg",
                    "/video.mjpg",
                    "/video.cgi",
                    "/snapshot.jpg",
                    "/image.jpg",
                    "/cam.jpg",
                ];
                
                for endpoint in endpoints {
                    if result.data.contains(endpoint) {
                        return Some(format!("http://{}:{}{}", result.ip, result.port, endpoint));
                    }
                }
                
                // Default MJPEG endpoint
                Some(format!("http://{}:{}/mjpeg", result.ip, result.port))
            }
            WebcamAccessType::RTSP => {
                Some(format!("rtsp://{}:{}/", result.ip, result.port))
            }
            WebcamAccessType::HTTP => {
                Some(format!("http://{}:{}/", result.ip, result.port))
            }
            WebcamAccessType::Unknown => {
                Some(format!("http://{}:{}/", result.ip, result.port))
            }
        }
    }

    /// Attempt to fetch an image from a remote webcam
    pub async fn fetch_webcam_image(&self, webcam: &RemoteWebcam) -> Result<Vec<u8>, ShodanError> {
        debug!("Fetching image from webcam: {}", webcam.url);

        let response = self.client
            .get(&webcam.url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        if response.status().is_success() {
            let bytes = response.bytes().await?;
            info!("Successfully fetched {} bytes from {}", bytes.len(), webcam.url);
            Ok(bytes.to_vec())
        } else {
            warn!("Failed to fetch image from {}: {}", webcam.url, response.status());
            Err(ShodanError::Generic(format!("HTTP {}", response.status())))
        }
    }
}

