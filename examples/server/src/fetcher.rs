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

    /// Resolve player textures (skin, cape) from official Mojang sessionserver API.
    pub async fn resolve_mojang_player(
        &self,
        name: &str,
    ) -> Result<(String, Option<String>, String), AppError> {
        let profile_url = format!("https://api.mojang.com/users/profiles/minecraft/{name}");
        let resp =
            self.client.get(&profile_url).send().await.map_err(|e| {
                AppError::FetchError(format!("Mojang profile API request failed: {e}"))
            })?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND
            || resp.status() == reqwest::StatusCode::NO_CONTENT
        {
            return Err(AppError::NotFound(format!(
                "Player {name} does not exist in Minecraft official server."
            )));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| AppError::FetchError(format!("Failed to read Mojang profile: {e}")))?;
        let info: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| {
            AppError::FetchError(format!("Failed to parse Mojang profile JSON: {e}"))
        })?;

        let uuid = info["id"]
            .as_str()
            .ok_or_else(|| AppError::NotFound(format!("Player {name} has no valid UUID")))?;
        let official_name = info["name"].as_str().unwrap_or(name).to_string();

        let session_url =
            format!("https://sessionserver.mojang.com/session/minecraft/profile/{uuid}");
        let session_resp =
            self.client.get(&session_url).send().await.map_err(|e| {
                AppError::FetchError(format!("Mojang session API request failed: {e}"))
            })?;

        if !session_resp.status().is_success() {
            return Err(AppError::NotFound(format!(
                "Failed to fetch textures for player {name}"
            )));
        }

        let session_bytes = session_resp
            .bytes()
            .await
            .map_err(|e| AppError::FetchError(format!("Failed to read session bytes: {e}")))?;
        let session_info: serde_json::Value = serde_json::from_slice(&session_bytes)
            .map_err(|e| AppError::FetchError(format!("Failed to parse session JSON: {e}")))?;

        let (skin_url, cape_url) = extract_textures_from_profile(&session_info)?;
        Ok((skin_url, cape_url, official_name))
    }

    /// Resolve player textures (skin, cape) from LittleSkin Yggdrasil API.
    pub async fn resolve_littleskin_player(
        &self,
        name: &str,
    ) -> Result<(String, Option<String>, String), AppError> {
        let profile_url =
            format!("https://littleskin.cn/api/yggdrasil/api/users/profiles/minecraft/{name}");
        let resp = self.client.get(&profile_url).send().await.map_err(|e| {
            AppError::FetchError(format!("LittleSkin profile API request failed: {e}"))
        })?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND
            || resp.status() == reqwest::StatusCode::NO_CONTENT
        {
            return Err(AppError::NotFound(format!(
                "Player {name} does not exist in LittleSkin."
            )));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| AppError::FetchError(format!("Failed to read LittleSkin profile: {e}")))?;
        let info: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| {
            AppError::FetchError(format!("Failed to parse LittleSkin profile JSON: {e}"))
        })?;

        let uuid = info["id"].as_str().ok_or_else(|| {
            AppError::NotFound(format!("Player {name} does not exist in LittleSkin."))
        })?;
        let official_name = info["name"].as_str().unwrap_or(name).to_string();

        let session_url = format!(
            "https://littleskin.cn/api/yggdrasil/sessionserver/session/minecraft/profile/{uuid}"
        );
        let session_resp = self.client.get(&session_url).send().await.map_err(|e| {
            AppError::FetchError(format!("LittleSkin session API request failed: {e}"))
        })?;

        if !session_resp.status().is_success() {
            return Err(AppError::NotFound(format!(
                "Failed to fetch textures for LittleSkin player {name}"
            )));
        }

        let session_bytes = session_resp.bytes().await.map_err(|e| {
            AppError::FetchError(format!("Failed to read LittleSkin session bytes: {e}"))
        })?;
        let session_info: serde_json::Value =
            serde_json::from_slice(&session_bytes).map_err(|e| {
                AppError::FetchError(format!("Failed to parse LittleSkin session JSON: {e}"))
            })?;

        let (skin_url, cape_url) = extract_textures_from_profile(&session_info)?;
        Ok((skin_url, cape_url, official_name))
    }
}

fn extract_textures_from_profile(
    session_info: &serde_json::Value,
) -> Result<(String, Option<String>), AppError> {
    let properties = session_info["properties"]
        .as_array()
        .ok_or_else(|| AppError::NotFound("Profile has no properties".to_string()))?;

    for prop in properties {
        if prop["name"].as_str() == Some("textures") {
            if let Some(val_b64) = prop["value"].as_str() {
                let decoded = base64::engine::general_purpose::STANDARD
                    .decode(val_b64)
                    .map_err(|e| AppError::FetchError(format!("Invalid texture base64: {e}")))?;
                let text_val: serde_json::Value = serde_json::from_slice(&decoded)
                    .map_err(|e| AppError::FetchError(format!("Invalid texture JSON: {e}")))?;

                let skin_url = text_val["textures"]["SKIN"]["url"]
                    .as_str()
                    .ok_or_else(|| {
                        AppError::NotFound("No skin texture found in profile".to_string())
                    })?
                    .to_string();

                let cape_url = text_val["textures"]["CAPE"]["url"]
                    .as_str()
                    .map(|s| s.to_string());

                return Ok((skin_url, cape_url));
            }
        }
    }

    Err(AppError::NotFound(
        "No texture property found in profile".to_string(),
    ))
}
