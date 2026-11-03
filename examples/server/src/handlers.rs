use std::sync::Arc;
use std::time::Instant;

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tokio::sync::oneshot;

use crate::error::AppError;
use crate::fetcher::TextureFetcher;
use crate::model::{RenderQuery, SceneRenderQuery};
use crate::worker::{JobPayload, RenderJob, RenderMode, RenderWorkerPool, SceneCharacterTextures};

pub struct AppState {
    pub fetcher: TextureFetcher,
    pub worker_pool: RenderWorkerPool,
    pub start_time: Instant,
}

pub async fn root_handler() -> impl IntoResponse {
    Json(json!({
        "status": 200,
        "msg": "SkinRenderMC"
    }))
}

pub async fn render_get_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<RenderQuery>,
) -> Result<Response, AppError> {
    handle_render(state, query, RenderMode::Single).await
}

pub async fn render_post_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<RenderQuery>,
    body: axum::body::Bytes,
) -> Result<Response, AppError> {
    let resolved_query = parse_post_query(query, &headers, body)?;
    handle_render(state, resolved_query, RenderMode::Single).await
}

fn parse_post_query(
    query: RenderQuery,
    headers: &HeaderMap,
    bytes: axum::body::Bytes,
) -> Result<RenderQuery, AppError> {
    if bytes.is_empty() {
        return Ok(query);
    }
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    let trimmed = bytes.iter().find(|&&b| !b.is_ascii_whitespace());
    if content_type.contains("application/json") || trimmed == Some(&b'{') {
        let body_q: RenderQuery = serde_json::from_slice(&bytes)
            .map_err(|e| AppError::BadRequest(format!("Invalid JSON body: {e}")))?;
        Ok(query.merge_with(body_q))
    } else {
        // Parse as application/x-www-form-urlencoded
        let body_str = std::str::from_utf8(&bytes)
            .map_err(|e| AppError::BadRequest(format!("Invalid UTF-8 body: {e}")))?;
        let uri = format!("/?{body_str}")
            .parse()
            .map_err(|e| AppError::BadRequest(format!("Invalid form query format: {e}")))?;
        match axum::extract::Query::<RenderQuery>::try_from_uri(&uri) {
            Ok(axum::extract::Query(form_q)) => Ok(query.merge_with(form_q)),
            Err(e) => Err(AppError::BadRequest(format!("Invalid form body data: {e}"))),
        }
    }
}

fn parse_view_mode(view: &str) -> Result<RenderMode, AppError> {
    match view.to_ascii_lowercase().as_str() {
        "front" => Ok(RenderMode::Front),
        "back" => Ok(RenderMode::Back),
        "both" => Ok(RenderMode::Both),
        other => Err(AppError::BadRequest(format!(
            "Invalid view '{other}', expected 'front', 'back', or 'both'"
        ))),
    }
}

/// Handler for GET /url/image/:view (front, back, both)
pub async fn url_image_get_handler(
    State(state): State<Arc<AppState>>,
    Path(view): Path<String>,
    Query(mut query): Query<RenderQuery>,
) -> Result<Response, AppError> {
    let mode = parse_view_mode(&view)?;
    query.format = Some("png".to_string());
    handle_render(state, query, mode).await
}

/// Handler for POST /url/image/:view
pub async fn url_image_post_handler(
    State(state): State<Arc<AppState>>,
    Path(view): Path<String>,
    headers: HeaderMap,
    Query(query): Query<RenderQuery>,
    body: axum::body::Bytes,
) -> Result<Response, AppError> {
    let mode = parse_view_mode(&view)?;
    let mut resolved_query = parse_post_query(query, &headers, body)?;
    resolved_query.format = Some("png".to_string());
    handle_render(state, resolved_query, mode).await
}

/// Handler for GET /url/json/all
pub async fn url_json_all_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<RenderQuery>,
) -> Result<Response, AppError> {
    handle_render(state, query, RenderMode::JsonAll).await
}

#[derive(Debug, serde::Deserialize)]
pub struct UsernameQuery {
    pub name: Option<String>,
    #[serde(flatten)]
    pub render_query: RenderQuery,
}

/// Handler for GET /mojang/image/:view?name=...
pub async fn mojang_image_handler(
    State(state): State<Arc<AppState>>,
    Path(view): Path<String>,
    Query(query): Query<UsernameQuery>,
) -> Result<Response, AppError> {
    let mode = parse_view_mode(&view)?;
    let name = query
        .name
        .as_deref()
        .filter(|n| !n.trim().is_empty())
        .ok_or_else(|| AppError::BadRequest("Missing required 'name' parameter".to_string()))?;

    let (skin_url, cape_url, official_name, is_slim) =
        state.fetcher.resolve_mojang_player(name).await?;
    let mut rq = query.render_query;
    rq.skin_url = Some(skin_url);
    if rq.cape_url.is_none() {
        rq.cape_url = cape_url;
    }
    if rq.nametag.is_none() {
        rq.nametag = Some(official_name);
    }
    if rq.slim.is_none() {
        rq.slim = Some(is_slim);
    }
    rq.format = Some("png".to_string());
    handle_render(state, rq, mode).await
}

/// Handler for GET /littleskin/image/:view?name=...
pub async fn littleskin_image_handler(
    State(state): State<Arc<AppState>>,
    Path(view): Path<String>,
    Query(query): Query<UsernameQuery>,
) -> Result<Response, AppError> {
    let mode = parse_view_mode(&view)?;
    let name = query
        .name
        .as_deref()
        .filter(|n| !n.trim().is_empty())
        .ok_or_else(|| AppError::BadRequest("Missing required 'name' parameter".to_string()))?;

    let (skin_url, cape_url, official_name, is_slim) =
        state.fetcher.resolve_littleskin_player(name).await?;
    let mut rq = query.render_query;
    rq.skin_url = Some(skin_url);
    if rq.cape_url.is_none() {
        rq.cape_url = cape_url;
    }
    if rq.nametag.is_none() {
        rq.nametag = Some(official_name);
    }
    if rq.slim.is_none() {
        rq.slim = Some(is_slim);
    }
    rq.format = Some("png".to_string());
    handle_render(state, rq, mode).await
}

async fn handle_render(
    state: Arc<AppState>,
    query: RenderQuery,
    mode: RenderMode,
) -> Result<Response, AppError> {
    let start = Instant::now();

    let skin_target = query
        .resolved_skin_url()
        .ok_or_else(|| AppError::BadRequest("Missing required 'skinUrl' parameter".to_string()))?;

    let cape_target = query.resolved_cape_url();
    let elytra_target = query.resolved_elytra_url();

    // Fetch required skin texture and optional accessories concurrently
    let skin_fut = state.fetcher.fetch(skin_target);
    let cape_fut = async {
        match cape_target {
            Some(t) => state.fetcher.fetch(t).await.map(Some),
            None => Ok(None),
        }
    };
    let elytra_fut = async {
        match elytra_target {
            Some(t) => state.fetcher.fetch(t).await.map(Some),
            None => Ok(None),
        }
    };

    let (skin_res, cape_res, elytra_res) = tokio::join!(skin_fut, cape_fut, elytra_fut);
    let skin_bytes = skin_res?;
    let cape_bytes = cape_res?;
    let elytra_bytes = elytra_res?;

    let (tx, rx) = oneshot::channel();
    let job = RenderJob {
        payload: crate::worker::JobPayload::Single {
            skin_bytes,
            cape_bytes,
            elytra_bytes,
            query,
            mode,
        },
        responder: tx,
    };

    state.worker_pool.submit(job)?;

    let (bytes, content_type) = rx
        .await
        .map_err(|_| AppError::Internal("Worker dropped render task".to_string()))??;

    let elapsed_ms = start.elapsed().as_millis();

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, content_type.parse().unwrap());
    let cache_control = if mode == RenderMode::JsonAll {
        "no-store, no-cache, must-revalidate"
    } else {
        "public, max-age=86400"
    };
    headers.insert(header::CACHE_CONTROL, cache_control.parse().unwrap());
    headers.insert("X-Render-Time-Ms", elapsed_ms.to_string().parse().unwrap());

    Ok((StatusCode::OK, headers, bytes).into_response())
}

pub async fn render_scene_handler(
    State(state): State<Arc<AppState>>,
    Json(query): Json<SceneRenderQuery>,
) -> Result<Response, AppError> {
    let start = Instant::now();

    if query.characters.is_empty() {
        return Err(AppError::BadRequest(
            "Scene must contain at least one character in 'characters'".to_string(),
        ));
    }

    let mut char_futs = Vec::with_capacity(query.characters.len());

    for char_q in &query.characters {
        let skin_target = char_q.resolved_skin_url().ok_or_else(|| {
            AppError::BadRequest("Each character must have a 'skin' or 'skinUrl'".to_string())
        })?;
        let cape_target = char_q.resolved_cape_url();
        let elytra_target = char_q.resolved_elytra_url();

        let fetcher = &state.fetcher;
        let skin_fut = fetcher.fetch(skin_target);
        let cape_fut = async move {
            match cape_target {
                Some(t) => fetcher.fetch(t).await.map(Some),
                None => Ok(None),
            }
        };
        let elytra_fut = async move {
            match elytra_target {
                Some(t) => fetcher.fetch(t).await.map(Some),
                None => Ok(None),
            }
        };

        char_futs.push(async move {
            let (skin_res, cape_res, elytra_res) = tokio::join!(skin_fut, cape_fut, elytra_fut);
            let skin_bytes = skin_res?;
            let cape_bytes = cape_res?;
            let elytra_bytes = elytra_res?;
            Result::<_, AppError>::Ok((skin_bytes, cape_bytes, elytra_bytes))
        });
    }

    let mut fetched_results = Vec::with_capacity(char_futs.len());
    for fut in char_futs {
        fetched_results.push(fut.await?);
    }

    let mut scene_characters = Vec::with_capacity(query.characters.len());
    for (char_q, (skin_bytes, cape_bytes, elytra_bytes)) in
        query.characters.clone().into_iter().zip(fetched_results)
    {
        scene_characters.push(SceneCharacterTextures {
            skin_bytes,
            cape_bytes,
            elytra_bytes,
            query: char_q,
        });
    }

    let (tx, rx) = oneshot::channel();
    let job = RenderJob {
        payload: JobPayload::Scene {
            characters: scene_characters,
            query,
        },
        responder: tx,
    };

    state.worker_pool.submit(job)?;

    let (bytes, content_type) = rx
        .await
        .map_err(|_| AppError::Internal("Worker dropped render task".to_string()))??;

    let elapsed_ms = start.elapsed().as_millis();

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, content_type.parse().unwrap());
    headers.insert(
        header::CACHE_CONTROL,
        "public, max-age=86400".parse().unwrap(),
    );
    headers.insert("X-Render-Time-Ms", elapsed_ms.to_string().parse().unwrap());

    Ok((StatusCode::OK, headers, bytes).into_response())
}

pub async fn health_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "uptime_secs": state.start_time.elapsed().as_secs(),
    }))
}
