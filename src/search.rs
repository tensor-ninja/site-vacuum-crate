use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, info};

use crate::models::SearchResult;

pub struct SearchEngine {
    client: Client,
    api_key: Option<String>,
    google_api_key: Option<String>,
    google_cx: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchApiResponse {
    organic_results: Vec<SearchItem>,
}

#[derive(Debug, Deserialize)]
struct SearchItem {
    title: String,
    snippet: String,
    link: String,
}

#[derive(Debug, Deserialize)]
struct GoogleSearchResponse {
    items: Option<Vec<GoogleSearchItem>>,
}

#[derive(Debug, Deserialize)]
struct GoogleSearchItem {
    title: String,
    snippet: String,
    link: String,
}

impl Default for SearchEngine {
    fn default() -> Self {
        let client = Client::builder()
            .user_agent("SiteVacuum Search/1.0")
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            client,
            api_key: std::env::var("SEARCH_API_KEY").ok(),
            google_api_key: std::env::var("GOOGLE_API_KEY").ok(),
            google_cx: std::env::var("GOOGLE_CX").ok(),
        }
    }
}

impl SearchEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        info!("Searching for: {} with limit {}", query, limit);
        if self.google_api_key.is_some() && self.google_cx.is_some() {
            self.search_with_google(query, limit).await
        } else if self.api_key.is_some() {
            // Fall back to another search API if available
            self.search_with_api(query, limit).await
        } else {
            Err(anyhow::anyhow!("No search API key or credentials provided"))
        }
    }

    async fn search_with_google(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let api_key = self.google_api_key.as_ref().unwrap();
        let cx = self.google_cx.as_ref().unwrap();
        let url = format!(
            "https://www.googleapis.com/customsearch/v1?key={}&cx={}&q={}&num={}",
            api_key,
            cx,
            urlencoding::encode(query),
            limit.min(10)
        );
        
        info!("Performing Google search query: {}", query);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            debug!("Google API error: {:?}", response.status());
            return Err(anyhow::anyhow!("Google API error: {}", response.status()));
        }
        let search_results: GoogleSearchResponse = response.json().await?;
        let results = match search_results.items {
            Some(items) => items
                .into_iter()
                .map(|item| SearchResult {
                    title: item.title,
                    description: item.snippet,
                    url: item.link,
                    content: None,
                })
                .collect(),
            None => Vec::new(),
        };
        
        Ok(results)
    }

    async fn search_with_api(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let api_key = self.api_key.as_ref().unwrap();
        let url = format!(
            "https://api.searchprovider.com/search?q={}&limit={}&api_key={}",
            urlencoding::encode(query),
            limit,
            api_key
        );
        
        let response = self.client.get(&url).send().await?;
        let search_results: SearchApiResponse = response.json().await?;
        
        let results = search_results
            .organic_results
            .into_iter()
            .map(|item| SearchResult {
                title: item.title,
                description: item.snippet,
                url: item.link,
                content: None,
            })
            .collect();
        
        Ok(results)
    }

} 