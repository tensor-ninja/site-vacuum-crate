use axum::{response::IntoResponse, Json};
use serde_json::json;

pub mod search;
pub mod crawl;


pub async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

pub async fn search_info() -> impl IntoResponse {
    let google_api_key = std::env::var("GOOGLE_API_KEY").ok();
    let google_cx = std::env::var("GOOGLE_CX").ok();
    let search_api_key = std::env::var("SEARCH_API_KEY").ok();
    
    let mask_key = |key: &Option<String>| -> String {
        match key {
            Some(k) if k.len() > 8 => {
                let prefix = &k[0..4];
                let suffix = &k[k.len()-4..];
                format!("{}...{}", prefix, suffix)
            },
            Some(_) => "Set but too short (check your configuration)".to_string(),
            None => "Not configured".to_string(),
        }
    };
    
    let search_provider = if google_api_key.is_some() && google_cx.is_some() {
        "Google Custom Search API"
    } else if search_api_key.is_some() {
        "Alternative Search API"
    } else {
        "Simulated Search (demo mode)"
    };
    
    Json(json!({
        "search_provider": search_provider,
        "google_api_key": mask_key(&google_api_key),
        "google_cx": mask_key(&google_cx),
        "alternative_api_key": mask_key(&search_api_key),
        "note": "For security, API keys are partially masked. This endpoint helps verify your configuration."
    }))
} 