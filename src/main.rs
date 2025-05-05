use std::net::SocketAddr;
use std::path::Path;
use std::fs::OpenOptions;
use std::io::Write;

use axum::{
    routing::{get, post},
    Router,
};
use dotenv::dotenv;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod crawler;
mod models;
mod search;


#[tokio::main]
async fn main() {
    
    dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Create a CORS layer
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(api::health_check))
        .route("/search", post(api::search::search))
        .route("/search-info", get(api::search_info))
        .route("/crawl", post(api::crawl::crawl))
        .layer(cors);

    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on {}", addr);

    match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => {
            if let Err(err) = axum::serve(listener, app).await {
                tracing::error!("Server error: {}", err);
            }
        },
        Err(err) => {
            tracing::error!("Failed to bind to address {}: {}", addr, err);
        }
    }
} 