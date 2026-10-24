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
use crate::handlers::{health_handler, render_get_handler, render_post_handler, AppState};
use crate::worker::RenderWorkerPool;

#[derive(Parser, Debug)]
#[command(author, version, about = "High-performance HTTP skin rendering daemon for Eidolon")]
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

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(health_handler))
        .route("/health", get(health_handler))
        .route("/render", get(render_get_handler).post(render_post_handler))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let bind_addr = format!("{}:{}", args.host, args.port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    info!("Eidolon HTTP server ready and listening on http://{}", bind_addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Eidolon HTTP server gracefully stopped");
    Ok(())
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
