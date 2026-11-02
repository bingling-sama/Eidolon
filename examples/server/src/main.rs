mod error;
mod fetcher;
mod handlers;
mod model;
mod worker;

use std::sync::Arc;
use std::time::Instant;

use axum::{routing::get, Router};
use clap::Parser;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::fetcher::TextureFetcher;
use crate::handlers::{
    health_handler, littleskin_image_handler, mojang_image_handler, render_get_handler,
    render_post_handler, root_handler, url_image_get_handler, url_image_post_handler,
    url_json_all_handler, AppState,
};
use crate::worker::RenderWorkerPool;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "High-performance HTTP skin rendering daemon for Eidolon"
)]
struct CliArgs {
    /// Host address to bind
    #[arg(long, default_value = "0.0.0.0", env = "EIDOLON_HOST")]
    host: String,

    /// Port to listen on
    #[arg(short, long, default_value_t = 8080, env = "EIDOLON_PORT")]
    port: u16,

    /// Number of dedicated GPU render worker threads
    #[arg(short, long, default_value_t = 2, env = "EIDOLON_WORKERS")]
    workers: usize,

    /// Maximum pending render queue capacity
    #[arg(long, default_value_t = 256, env = "EIDOLON_QUEUE_CAPACITY")]
    queue_capacity: usize,

    /// Maximum in-memory texture cache entries
    #[arg(long, default_value_t = 2048, env = "EIDOLON_CACHE_ENTRIES")]
    cache_entries: u64,

    /// Texture cache time-to-live in seconds
    #[arg(long, default_value_t = 3600, env = "EIDOLON_CACHE_TTL")]
    cache_ttl: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "eidolon_server=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = CliArgs::parse();

    info!(
        "Starting Eidolon HTTP Server on {}:{} with {} render worker(s)",
        args.host, args.port, args.workers
    );

    let fetcher = TextureFetcher::new(args.cache_entries, args.cache_ttl);
    let worker_pool = RenderWorkerPool::new(args.workers.max(1), args.queue_capacity.max(1));

    let state = Arc::new(AppState {
        fetcher,
        worker_pool,
        start_time: Instant::now(),
    });

    let app = create_app(state);

    let bind_addr = format!("{}:{}", args.host, args.port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    info!(
        "Eidolon HTTP server ready and listening on http://{}",
        bind_addr
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Eidolon HTTP server gracefully stopped");
    Ok(())
}

pub fn create_app(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/", get(root_handler))
        .route("/health", get(health_handler))
        .route("/render", get(render_get_handler).post(render_post_handler))
        .route(
            "/url/image/{view}",
            get(url_image_get_handler).post(url_image_post_handler),
        )
        .route("/url/json/all", get(url_json_all_handler))
        .route("/mojang/image/{view}", get(mojang_image_handler))
        .route("/littleskin/image/{view}", get(littleskin_image_handler))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_http_api_endpoints() {
        let fetcher = TextureFetcher::new(100, 60);
        let worker_pool = RenderWorkerPool::new(0, 10);
        let state = Arc::new(AppState {
            fetcher,
            worker_pool,
            start_time: Instant::now(),
        });

        let app = create_app(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind ephemeral test port");
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let client = reqwest::Client::new();

        // 1. GET /
        let res = client.get(format!("http://{addr}/")).send().await.unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);
        let body_bytes = res.bytes().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["status"], 200);
        assert_eq!(body["msg"], "SkinRenderMC");

        // 2. GET /health
        let res = client
            .get(format!("http://{addr}/health"))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);
        let body_bytes = res.bytes().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["status"], "ok");

        // 3. GET /url/image/invalid
        let res = client
            .get(format!("http://{addr}/url/image/unknown_view"))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::BAD_REQUEST);

        // 4. GET /mojang/image/front without name parameter
        let res = client
            .get(format!("http://{addr}/mojang/image/front"))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::BAD_REQUEST);

        // 5. GET /littleskin/image/both without name parameter
        let res = client
            .get(format!("http://{addr}/littleskin/image/both"))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::BAD_REQUEST);

        // 6. POST /url/image/front with JSON body missing skinUrl
        let res = client
            .post(format!("http://{addr}/url/image/front"))
            .header("Content-Type", "application/json")
            .body("{}")
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::BAD_REQUEST);

        // 7. POST /url/image/both with form urlencoded body missing skinUrl
        let res = client
            .post(format!("http://{addr}/url/image/both"))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body("definition=2.0")
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::BAD_REQUEST);
    }
}
