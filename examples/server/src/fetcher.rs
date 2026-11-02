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

    /// Resolve player textures (skin, cape) and model type from official Mojang sessionserver API.
    pub async fn resolve_mojang_player(
        &self,
        name: &str,
    ) -> Result<(String, Option<String>, String, bool), AppError> {
        let encoded_name = url_encode(name.trim());
        let profile_url = format!("https://api.mojang.com/users/profiles/minecraft/{encoded_name}");
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

        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(AppError::FetchError(
                "Mojang profile API rate limit exceeded (HTTP 429)".to_string(),
            ));
        }

        if !resp.status().is_success() {
            return Err(AppError::FetchError(format!(
                "Mojang profile API returned error HTTP {}: {}",
                resp.status(),
                resp.status().canonical_reason().unwrap_or("")
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

        if session_resp.status() == reqwest::StatusCode::NOT_FOUND
            || session_resp.status() == reqwest::StatusCode::NO_CONTENT
        {
            return Err(AppError::NotFound(format!(
                "Player {name} profile not found in Mojang session server."
            )));
        }

        if session_resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(AppError::FetchError(
                "Mojang session API rate limit exceeded (HTTP 429)".to_string(),
            ));
        }

        if !session_resp.status().is_success() {
            return Err(AppError::FetchError(format!(
                "Mojang session API returned error HTTP {}: {}",
                session_resp.status(),
                session_resp.status().canonical_reason().unwrap_or("")
            )));
        }

        let session_bytes = session_resp
            .bytes()
            .await
            .map_err(|e| AppError::FetchError(format!("Failed to read session bytes: {e}")))?;
        let session_info: serde_json::Value = serde_json::from_slice(&session_bytes)
            .map_err(|e| AppError::FetchError(format!("Failed to parse session JSON: {e}")))?;

        let (skin_url, cape_url, is_slim) = extract_textures_from_profile(&session_info, uuid)?;
        Ok((skin_url, cape_url, official_name, is_slim))
    }

    /// Resolve player textures (skin, cape) and model type from LittleSkin Yggdrasil API.
    pub async fn resolve_littleskin_player(
        &self,
        name: &str,
    ) -> Result<(String, Option<String>, String, bool), AppError> {
        let encoded_name = url_encode(name.trim());
        let profile_url = format!(
            "https://littleskin.cn/api/yggdrasil/api/users/profiles/minecraft/{encoded_name}"
        );
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

        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(AppError::FetchError(
                "LittleSkin profile API rate limit exceeded (HTTP 429)".to_string(),
            ));
        }

        if !resp.status().is_success() {
            return Err(AppError::FetchError(format!(
                "LittleSkin profile API returned error HTTP {}: {}",
                resp.status(),
                resp.status().canonical_reason().unwrap_or("")
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

        if session_resp.status() == reqwest::StatusCode::NOT_FOUND
            || session_resp.status() == reqwest::StatusCode::NO_CONTENT
        {
            return Err(AppError::NotFound(format!(
                "LittleSkin player {name} session profile not found."
            )));
        }

        if session_resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(AppError::FetchError(
                "LittleSkin session API rate limit exceeded (HTTP 429)".to_string(),
            ));
        }

        if !session_resp.status().is_success() {
            return Err(AppError::FetchError(format!(
                "LittleSkin session API returned error HTTP {}: {}",
                session_resp.status(),
                session_resp.status().canonical_reason().unwrap_or("")
            )));
        }

        let session_bytes = session_resp.bytes().await.map_err(|e| {
            AppError::FetchError(format!("Failed to read LittleSkin session bytes: {e}"))
        })?;
        let session_info: serde_json::Value =
            serde_json::from_slice(&session_bytes).map_err(|e| {
                AppError::FetchError(format!("Failed to parse LittleSkin session JSON: {e}"))
            })?;

        let (skin_url, cape_url, is_slim) = extract_textures_from_profile(&session_info, uuid)?;
        Ok((skin_url, cape_url, official_name, is_slim))
    }
}

fn extract_textures_from_profile(
    session_info: &serde_json::Value,
    uuid: &str,
) -> Result<(String, Option<String>, bool), AppError> {
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

                let (skin_url, is_slim) = if let Some(skin_url) =
                    text_val["textures"]["SKIN"]["url"].as_str()
                {
                    let is_slim = text_val["textures"]["SKIN"]["metadata"]["model"]
                        .as_str()
                        .map(|m| m.eq_ignore_ascii_case("slim"))
                        .unwrap_or(false);
                    (skin_url.to_string(), is_slim)
                } else {
                    // Fallback to official default Steve/Alex texture based on player UUID
                    let is_slim = default_model_is_alex(uuid);
                    let fallback_url = if is_slim {
                        "https://textures.minecraft.net/texture/88a95d732e79ced2b24b426e2e5055fa08f1b626d37fa10b93081e646fd1a46b".to_string()
                    } else {
                        "https://textures.minecraft.net/texture/1a4af718455d4aab528e7a61f86fa25e6a369d1768dcb13f7df319a71824766".to_string()
                    };
                    (fallback_url, is_slim)
                };

                let cape_url = text_val["textures"]["CAPE"]["url"]
                    .as_str()
                    .map(|s| s.to_string());

                return Ok((skin_url, cape_url, is_slim));
            }
        }
    }

    // If profile has no textures property, fallback to default Steve/Alex based on UUID
    let is_slim = default_model_is_alex(uuid);
    let fallback_url = if is_slim {
        "https://textures.minecraft.net/texture/88a95d732e79ced2b24b426e2e5055fa08f1b626d37fa10b93081e646fd1a46b".to_string()
    } else {
        "https://textures.minecraft.net/texture/1a4af718455d4aab528e7a61f86fa25e6a369d1768dcb13f7df319a71824766".to_string()
    };
    Ok((fallback_url, None, is_slim))
}

pub fn default_model_is_alex(uuid: &str) -> bool {
    let clean = uuid.replace('-', "");
    if clean.len() != 32 {
        return false;
    }
    if let (Ok(msb), Ok(lsb)) = (
        u64::from_str_radix(&clean[0..16], 16),
        u64::from_str_radix(&clean[16..32], 16),
    ) {
        let hash = (msb >> 32) ^ msb ^ (lsb >> 32) ^ lsb;
        (hash & 1) == 1
    } else {
        false
    }
}

fn url_encode(input: &str) -> String {
    let mut encoded = String::new();
    for byte in input.bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                use std::fmt::Write;
                let _ = write!(encoded, "%{:02X}", byte);
            }
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_encode() {
        assert_eq!(url_encode("Steve"), "Steve");
        assert_eq!(url_encode("Steve 1"), "Steve%201");
        assert_eq!(url_encode("Alex/Steve"), "Alex%2FSteve");
        assert_eq!(url_encode("冰"), "%E5%86%B0");
    }

    #[test]
    fn test_default_model_is_alex() {
        // Test deterministic UUID hash calculation
        let alex_uuid = "00000000000000000000000000000001";
        let steve_uuid = "00000000000000000000000000000002";
        assert!(default_model_is_alex(alex_uuid));
        assert!(!default_model_is_alex(steve_uuid));
    }

    #[test]
    fn test_extract_textures_missing_skin_fallback() {
        let alex_uuid = "00000000000000000000000000000001";
        // Profile with empty properties
        let empty_profile = serde_json::json!({
            "id": alex_uuid,
            "name": "TestAlex",
            "properties": []
        });

        let (skin_url, cape_url, is_slim) =
            extract_textures_from_profile(&empty_profile, alex_uuid).unwrap();
        assert!(skin_url.contains("textures.minecraft.net"));
        assert_eq!(cape_url, None);
        assert!(is_slim);
    }
}
