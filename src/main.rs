use axum::{
    http::StatusCode, 
    routing::{get, post}, 
    Router,
};
use serde::{Deserialize, Serialize};
use axum::Json;
use axum_macros::debug_handler;
use tracing::{info, error, debug, instrument, Level};
use tracing_subscriber::{FmtSubscriber, EnvFilter};
use tokio::sync::broadcast;
use axum::response::sse::{Event, Sse};
use futures::stream::{Stream, StreamExt};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio_stream::wrappers::BroadcastStream;
use tower_http::cors::{Any, CorsLayer};

pub mod crawler;
pub mod converter;
pub mod fetcher;

use crawler::{Crawler, CrawlerEvent};

// Create a type alias for our event bus
type EventBus = broadcast::Sender<CrawlerEvent>;

#[tokio::main]
async fn main() {
    // Initialize the logger
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .with_target(true)
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global default subscriber");
    
    info!("Starting site-vacuum server");
    
    // Create our event bus with a channel capacity of 100 events
    let (tx, _rx) = broadcast::channel::<CrawlerEvent>(100);
    let event_bus = Arc::new(tx);
    
    // Configure CORS to allow all origins
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    let app = Router::new()
        .route("/", get(index))
        .route("/crawl", post(crawl_site))
        .route("/events", get(sse_handler))
        .with_state(event_bus)
        .layer(cors);
    
    let addr = "0.0.0.0:8000";
    info!("Listening on {}", addr);
    info!("Starting site-vacuum server");
    info!("/crawl (POST) - Crawl a website and return markdown");
    info!("/events (GET) - SSE endpoint for crawler updates");
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn index() -> &'static str {
    debug!("Handling index request");
    "hello, world!"
}

#[debug_handler]
#[instrument(skip(payload, event_bus), fields(url = %payload.url, concurrent = %payload.concurrent, js_rendering = %payload.enable_js_rendering))]
async fn crawl_site(
    axum::extract::State(event_bus): axum::extract::State<Arc<EventBus>>,
    Json(payload): Json<CrawlRequest>
) -> (StatusCode, Json<CrawlResponse>) {
    info!("Starting crawl for {} with JS rendering: {}", payload.url, payload.enable_js_rendering);
    
    let crawler = match Crawler::new(&payload.url, payload.concurrent, payload.enable_js_rendering, event_bus) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create crawler: {:?}", e);
            return (StatusCode::BAD_REQUEST, Json(CrawlResponse { markdown: "Failed to create crawler".to_string() }));
        }
    };
    
    if let Err(e) = crawler.crawl().await {
        error!("Crawl failed: {:?}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(CrawlResponse { markdown: "Crawl failed".to_string() }));
    }
    
    match crawler.generate_markdown().await {
        Ok(markdown) => {
            info!("Successfully generated markdown for {}", payload.url);
            let response = CrawlResponse { markdown };
            (StatusCode::CREATED, Json(response))
        },
        Err(e) => {
            error!("Failed to generate markdown: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(CrawlResponse { markdown: "Failed to generate markdown".to_string() }))
        }
    }
}

// SSE handler that streams crawler events to clients
async fn sse_handler(
    axum::extract::State(event_bus): axum::extract::State<Arc<EventBus>>
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // Create a new receiver from our sender
    let rx = event_bus.subscribe();
    
    // Convert the broadcast receiver into a stream
    let stream = BroadcastStream::new(rx)
        .map(|msg| {
            match msg {
                Ok(event) => {
                    // Convert our crawler event to an SSE event
                    let data = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
                    Ok(Event::default().data(data))
                },
                Err(_) => {
                    // Handle potential broadcast errors with an empty event
                    Ok(Event::default().data("{}"))
                }
            }
        });
    
    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping")
    )
}

#[derive(Deserialize)]
struct CrawlRequest {
    url: String,
    concurrent: u32,
    #[serde(default)]
    enable_js_rendering: bool,
}

#[derive(Serialize)]
struct CrawlResponse {
    markdown: String,
}
