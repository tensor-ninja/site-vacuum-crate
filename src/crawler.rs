use anyhow::{Context, Result};
use futures::stream::{FuturesUnordered, StreamExt};
use scraper::{Html, Selector};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore, broadcast};
use url::Url;
use tracing::{info, error, debug};

use crate::converter::convert_to_markdown;
use crate::fetcher::smart_fetch_url;

#[derive(Debug, Clone, Serialize)]
pub enum CrawlerEventType {
    Started,
    VisitingUrl,
    UrlProcessed,
    Completed,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct CrawlerEvent {
    pub event_type: CrawlerEventType,
    pub url: String,
    pub message: Option<String>,
    pub timestamp: i64,
}

pub struct Crawler {
    base_url: Url,
    visited_urls: Arc<Mutex<HashSet<String>>>,
    content_map: Arc<Mutex<HashMap<String, String>>>,
    max_concurrent_requests: u32,
    semaphore: Arc<Semaphore>,
    enable_js_rendering: bool,
    event_bus: Arc<broadcast::Sender<CrawlerEvent>>,
}


impl Crawler {
    pub fn new(
        url: &str, 
        max_concurrent_requests: u32, 
        enable_js_rendering: bool,
        event_bus: Arc<broadcast::Sender<CrawlerEvent>>
    ) -> Result<Self> {
        let base_url = Url::parse(url).context("Failed to parse base URL")?;
        info!("Created crawler for base URL: {}", base_url);

        // Emit started event
        let start_event = CrawlerEvent {
            event_type: CrawlerEventType::Started,
            url: base_url.to_string(),
            message: Some(format!("Starting crawl for {}", base_url)),
            timestamp: chrono::Utc::now().timestamp(),
        };
        let _ = event_bus.send(start_event);

        Ok(Self {
            base_url,
            visited_urls: Arc::new(Mutex::new(HashSet::new())),
            content_map: Arc::new(Mutex::new(HashMap::new())),
            max_concurrent_requests,
            semaphore: Arc::new(Semaphore::new(max_concurrent_requests as usize)),
            enable_js_rendering,
            event_bus,
        })
    }

    pub async fn crawl(&self) -> Result<()> {
        let mut queue = vec![self.base_url.clone()];
        let mut tasks = FuturesUnordered::new();
        
        info!("Starting crawl with {} concurrent requests", self.max_concurrent_requests);

        while !queue.is_empty() || !tasks.is_empty() {
            // Log queue status
            if !queue.is_empty() {
                debug!("Queue size: {}, Tasks in progress: {}", queue.len(), tasks.len());
            }
            
            // Fill tasks queue with URLs to process
            while !queue.is_empty() && tasks.len() < self.max_concurrent_requests as usize {
                let url = queue.pop().unwrap();
                let url_str = url.to_string();

                // Skip already visited URLs
                if self.visited_urls.lock().await.contains(&url_str) {
                    debug!("Skipping already visited URL: {}", url_str);
                    continue;
                }

                // Mark as visited
                self.visited_urls.lock().await.insert(url_str.clone());
                info!("Queueing URL for processing: {}", url_str);

                // Emit visiting URL event
                let visiting_event = CrawlerEvent {
                    event_type: CrawlerEventType::VisitingUrl,
                    url: url_str.clone(),
                    message: Some(format!("Processing URL: {}", url_str)),
                    timestamp: chrono::Utc::now().timestamp(),
                };
                let _ = self.event_bus.send(visiting_event);

                // Clone necessary values for the async task
                let base_url = self.base_url.clone();
                let content_map = Arc::clone(&self.content_map);
                let semaphore = Arc::clone(&self.semaphore);
                let enable_js_rendering = self.enable_js_rendering;
                let event_bus = Arc::clone(&self.event_bus);

                tasks.push(async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    // Fetch and process URL with JS rendering flag
                    match Self::process_url(&url, &base_url, &content_map, enable_js_rendering).await {
                        Ok(new_urls) => {
                            // Emit URL processed event
                            let processed_event = CrawlerEvent {
                                event_type: CrawlerEventType::UrlProcessed,
                                url: url_str.clone(),
                                message: Some(format!("Processed URL: {} (found {} links)", url_str, new_urls.len())),
                                timestamp: chrono::Utc::now().timestamp(),
                            };
                            let _ = event_bus.send(processed_event);
                            (url, new_urls, None)
                        },
                        Err(e) => {
                            // Emit error event
                            let error_event = CrawlerEvent {
                                event_type: CrawlerEventType::Error,
                                url: url_str.clone(),
                                message: Some(format!("Error processing {}: {}", url_str, e)),
                                timestamp: chrono::Utc::now().timestamp(),
                            };
                            let _ = event_bus.send(error_event);
                            (url, vec![], Some(e))
                        }
                    }
                });
            }

            // Process completed tasks
            if let Some((url, new_urls, error)) = tasks.next().await {
                if let Some(e) = error {
                    error!("Error processing {}: {}", url, e);
                } else {
                    info!("Processed URL: {} (found {} links)", url, new_urls.len());
                }

                // Add new URLs to the queue
                for new_url in new_urls {
                    let new_url_str = new_url.to_string();
                    if !self
                        .visited_urls
                        .lock()
                        .await
                        .contains(&new_url_str)
                    {
                        debug!("Discovered new URL: {}", new_url_str);
                        queue.push(new_url);
                    }
                }
            }
        }

        info!("Crawl completed. Visited {} pages", self.visited_urls.lock().await.len());
        
        // Emit completed event
        let completed_event = CrawlerEvent {
            event_type: CrawlerEventType::Completed,
            url: self.base_url.to_string(),
            message: Some(format!("Crawl completed. Visited {} pages", self.visited_urls.lock().await.len())),
            timestamp: chrono::Utc::now().timestamp(),
        };
        let _ = self.event_bus.send(completed_event);
        
        Ok(())
    }

    async fn process_url(
        url: &Url,
        base_url: &Url,
        content_map: &Arc<Mutex<HashMap<String, String>>>,
        enable_js_rendering: bool,
    ) -> Result<Vec<Url>> {
        // Fetch HTML content with JS rendering if enabled
        let html = smart_fetch_url(url, enable_js_rendering).await?;
        
        // Extract links first - before any await points
        let mut new_urls = Vec::new();
        {
            // Parse HTML - in its own scope so it's dropped before any awaits
            let document = Html::parse_document(&html);
            
            // Extract links while document is in scope
            let link_selector = Selector::parse("a[href]").unwrap();
            for element in document.select(&link_selector) {
                if let Some(href) = element.value().attr("href") {
                    if let Ok(mut new_url) = url.join(href) {
                        // Only follow URLs with the same domain
                        if new_url.domain() == base_url.domain() {
                            // Remove fragment
                            new_url.set_fragment(None);
                            // Remove query
                            new_url.set_query(None);
                            
                            new_urls.push(new_url);
                        }
                    }
                }
            }
        }
        
        // Convert to markdown
        let markdown = convert_to_markdown(&html);
        
        // Save content after link extraction
        let path = url.path().to_string();
        content_map.lock().await.insert(path, markdown);

        Ok(new_urls)
    }

    pub async fn generate_markdown(&self) -> Result<String> {
        let content_map = self.content_map.lock().await;
        let mut markdown = String::new();

        // Start with the root URL
        let root_path = self.base_url.path().to_string();
        if let Some(content) = content_map.get(&root_path) {
            markdown.push_str(&format!("# {}\n\n", self.base_url));
            markdown.push_str(content);
            markdown.push_str("\n\n");
        }

        // Add other pages in a structured manner
        for (path, content) in content_map.iter() {
            if path != &root_path {
                markdown.push_str(&format!("## {}\n\n", path));
                markdown.push_str(content);
                markdown.push_str("\n\n");
            }
        }

        Ok(markdown)
    }
}
