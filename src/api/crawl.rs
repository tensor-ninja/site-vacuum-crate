use axum::{extract::Json, http::StatusCode, response::IntoResponse};
use serde_json::json;
use tracing::{error, info};

use crate::{
    crawler::Crawler,
    models::{CrawlRequest, SearchResult},
};

pub async fn crawl(Json(req): Json<CrawlRequest>) -> impl IntoResponse {
    info!("Crawl request received for URL: {}", req.url);
    let mut crawler = Crawler::new();

    if let Some(headers) = &req.headers {
        crawler.headers = Some(headers.clone());
    }

    let crawl_results = match crawler.crawl_url(&req.url, req.limit).await {
        Ok(results) => results,
        Err(err) => {
            error!("Crawl error: {}", err);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": format!("Crawling failed: {}", err)
                })),
            );
        }
    };

    match req.format.as_str() {
        "json" => {
            (StatusCode::OK, Json(json!({
                "results": crawl_results,
                "url": req.url,
                "count": crawl_results.len()
            })))
        }
        "markdown" => {
            let markdown = format_as_markdown(&req.url, &crawl_results);
            (
                StatusCode::OK,
                Json(json!({
                    "markdown": markdown,
                    "url": req.url,
                    "count": crawl_results.len()
                })),
            )
        }
        _ => {
            (StatusCode::OK, Json(json!({
                "results": crawl_results,
                "url": req.url,
                "count": crawl_results.len()
            })))
        }
    }
}

fn format_as_markdown(url: &str, results: &[SearchResult]) -> String {
    let mut markdown = format!("# Crawl Results for \"{}\"\n\n", url);

    for (i, result) in results.iter().enumerate() {
        markdown.push_str(&format!("## {}. [{}]({})\n\n", i + 1, result.title, result.url));
        markdown.push_str(&format!("{}\n\n", result.description));
        
        if let Some(content) = &result.content {
            markdown.push_str("### Content Preview\n\n");
            
            // Convert HTML to Markdown
            let md_content = html2md::parse_html(content);
            
            // Limit preview to first 500 chars
            let preview = if md_content.len() > 500 {
                format!("{}...", &md_content[0..497])
            } else {
                md_content
            };
            
            markdown.push_str(&format!("{}\n\n", preview));
        }
    }

    markdown
} 