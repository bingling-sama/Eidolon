use std::sync::Arc;
use std::time::Instant;

use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tokio::sync::oneshot;

use crate::error::AppError;
use crate::fetcher::TextureFetcher;
use crate::model::RenderQuery;
use crate::worker::{RenderJob, RenderMode, RenderWorkerPool};

pub struct AppState {
    pub fetcher: TextureFetcher,
    pub worker_pool: RenderWorkerPool,
    pub start_time: Instant,
}

pub async fn render_get_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<RenderQuery>,
) -> Result<Response, AppError> {
    handle_render(state, query, RenderMode::Single).await
}

pub async fn render_post_handler(
    State(state): State<Arc<AppState>>,
    Json(query): Json<RenderQuery>,
) -> Result<Response, AppError> {
    handle_render(state, query, RenderMode::Single).await
}

/// Handler specifically for GET /url/image/both (SkinRenderMC API specification)
pub async fn render_both_get_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<RenderQuery>,
) -> Result<Response, AppError> {
    handle_render(state, query, RenderMode::Both).await
}

/// Handler for POST /url/image/both
pub async fn render_both_post_handler(
    State(state): State<Arc<AppState>>,
    Json(query): Json<RenderQuery>,
) -> Result<Response, AppError> {
    handle_render(state, query, RenderMode::Both).await
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
        skin_bytes,
        cape_bytes,
        elytra_bytes,
        query,
        mode,
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
