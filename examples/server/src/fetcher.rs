use std::path::Path;
use std::time::Duration;

use base64::Engine;
use moka::future::Cache;
use tracing::{debug, warn};

use crate::error::AppError;

const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024; // 10 MB

#[derive(Clone)]
pub struct TextureFetcher {
    client: reqwest::Client,
    cache: Cache<String, Vec<u8>>,
}

impl TextureFetcher {
    pub fn new(max_cache_entries: u64, ttl_secs: u64) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("Eidolon-Render-Server/1.0")
            .build()
            .expect("Failed to initialize HTTP client");

        let cache = Cache::builder()
            .max_capacity(max_cache_entries)
            .time_to_live(Duration::from_secs(ttl_secs))
            .build();

        Self { client, cache }
    }

    pub async fn fetch(&self, target: &str) -> Result<Vec<u8>, AppError> {
        let target = target.trim();
        if target.is_empty() {
            return Err(AppError::BadRequest("Target URL is empty".to_string()));
        }

        // 1. Data URI (e.g. data:image/png;base64,...)
        if let Some(rest) = target.strip_prefix("data:") {
            if let Some((_, data)) = rest.split_once(";base64,") {
                return base64::engine::general_purpose::STANDARD
                    .decode(data.trim())
                    .map_err(|e| AppError::BadRequest(format!("Invalid base64 data URI: {e}")));
            }
        }

        // 2. HTTP / HTTPS URL
        if target.starts_with("http://") || target.starts_with("https://") {
            if let Some(cached) = self.cache.get(target).await {
                debug!("Texture cache hit: {}", target);
                return Ok(cached);
            }

            debug!("Fetching texture from network: {}", target);
            let response = self.client.get(target).send().await.map_err(|e| {
                AppError::FetchError(format!("Failed to request texture '{target}': {e}"))
            })?;

            if !response.status().is_success() {
                return Err(AppError::FetchError(format!(
                    "Failed to fetch texture '{target}': HTTP {}",
                    response.status()
                )));
            }

            let bytes = response.bytes().await.map_err(|e| {
                AppError::FetchError(format!("Failed to read response body for '{target}': {e}"))
            })?;

            if bytes.len() > MAX_IMAGE_BYTES {
                return Err(AppError::BadRequest(format!(
                    "Texture '{target}' exceeds maximum size of 10MB"
                )));
            }

            let vec = bytes.to_vec();
            self.cache.insert(target.to_string(), vec.clone()).await;
            return Ok(vec);
        }

        // 3. Local file path
        let path = Path::new(target);
        if path.exists() && path.is_file() {
            debug!("Loading texture from local file: {}", target);
            return tokio::fs::read(path).await.map_err(|e| {
                AppError::FetchError(format!("Failed to read local file '{target}': {e}"))
            });
        }

        warn!("Unrecognized texture target: {}", target);
        Err(AppError::BadRequest(format!(
            "Target must be an http(s) URL, a base64 data URI, or a valid local file: '{target}'"
        )))
    }
}
