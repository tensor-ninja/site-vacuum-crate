use axum::{extract::Json, http::StatusCode, response::IntoResponse};
use serde_json::json;
use tracing::error;

use crate::{
    crawler::Crawler,
    models::{SearchRequest, SearchResponse, SearchResult},
    search::SearchEngine,
};

pub async fn search(Json(req): Json<SearchRequest>) -> impl IntoResponse {

    let search_engine = SearchEngine::new();
    let mut crawler = Crawler::new();

    if let Some(headers) = &req.headers {
        crawler.headers = Some(headers.clone());
    }

    let search_results = match search_engine.search(&req.search, req.search_limit).await {
        Ok(results) => results,
        Err(err) => {
            error!("Search error: {}", err);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": format!("Search failed: {}", err)
                })),
            );
        }
    };

    let results = if req.fetch_page_content {
        let urls: Vec<String> = search_results
            .iter()
            .map(|result| result.url.clone())
            .collect();

        let content_limit = if req.limit == 0 { 1 } else { req.limit };
        
        match crawler.crawl_urls(&urls, content_limit).await {
            Ok(crawl_results) => crawl_results,
            Err(err) => {
                error!("Crawl error: {}", err);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": format!("Content crawling failed: {}", err)
                    })),
                );
            }
        }
    } else {
        search_results
    };

    match req.return_format.as_str() {
        "json" => {
            let response = SearchResponse {
                results: results.clone(),
                query: req.search.clone(),
                count: results.len(),
            };
            (StatusCode::OK, Json(json!(response)))
        }
        "markdown" => {
            let markdown = format_as_markdown(&req.search, &results);
            (
                StatusCode::OK,
                Json(json!({
                    "markdown": markdown,
                    "query": req.search,
                    "count": results.len()
                })),
            )
        }
        _ => {
            let response = SearchResponse {
                results: results.clone(),
                query: req.search.clone(),
                count: results.len(),
            };
            (StatusCode::OK, Json(json!(response)))
        }
    }
}

fn format_as_markdown(query: &str, results: &[SearchResult]) -> String {
    let mut markdown = format!("# Search Results for \"{}\"\n\n", query);

    for (i, result) in results.iter().enumerate() {
        markdown.push_str(&format!("## {}. [{}]({})\n\n", i + 1, result.title, result.url));
        markdown.push_str(&format!("{}\n\n", result.description));
        
        if let Some(content) = &result.content {
            markdown.push_str("### Content Preview\n\n");
            
            let md_content = html2md::parse_html(content);
            
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