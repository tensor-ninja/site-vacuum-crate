use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchRequest {
    /// The search query to execute
    pub search: String,
    /// Maximum number of pages to crawl per domain (default: 0 = crawl all)
    #[serde(default)]
    pub limit: usize,
    /// Number of top results to return
    #[serde(default = "default_search_limit")]
    pub search_limit: usize,
    /// Choose output format ("markdown", "json", etc.)
    #[serde(default = "default_return_format")]
    pub return_format: String,
    /// If true, the search will perform a crawl to gather the content
    #[serde(default)]
    pub fetch_page_content: bool,
    /// If true, data will be cached for reuse
    #[serde(default)]
    pub store_data: bool,
    /// Custom headers to add to requests
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub title: String,
    pub description: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub query: String,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CrawlRequest {
    /// The URL to crawl
    pub url: String,
    /// Maximum number of pages to crawl (default: 1)
    #[serde(default = "default_crawl_limit")]
    pub limit: usize,
    /// Choose output format ("markdown", "json")
    #[serde(default = "default_return_format")]
    pub format: String,
    /// Custom headers to add to requests
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
}

fn default_search_limit() -> usize {
    10
}

fn default_return_format() -> String {
    "json".to_string()
}

fn default_crawl_limit() -> usize {
    1
} 