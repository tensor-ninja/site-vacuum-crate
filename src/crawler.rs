use anyhow::Result;
use spider::website::Website;
use std::collections::HashMap;
use scraper;

use crate::models::SearchResult;

pub struct Crawler {
    pub headers: Option<HashMap<String, String>>,
}

impl Default for Crawler {
    fn default() -> Self {
        Self {
            headers: None,
        }
    }
}

impl Crawler {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn crawl_url(&self, url: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let mut website = Website::new(url).with_depth(limit).build().unwrap();

        website.scrape().await;
        
        if let Some(pages) = website.get_pages() {
            let results = pages.iter().map(|page| {
                SearchResult {
                    title: extract_title(&page.get_html()),
                    description: extract_description(&page.get_html()),
                    url: page.get_url_final().to_string(),
                    content: Some(page.get_html()),
                }
            }).collect();
            Ok(results)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn crawl_urls(&self, urls: &[String], limit: usize) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();
        for url in urls {
            let result = self.crawl_url(url, limit).await?;
            results.extend(result);
        }
        Ok(results)
    }
}

fn extract_title(html: &str) -> String {
    if html.is_empty() {
        return "Unknown Title".to_string();
    }

    let document = scraper::Html::parse_document(html);
    let title_selector = scraper::Selector::parse("title").unwrap();
    
    document
        .select(&title_selector)
        .next()
        .and_then(|element| element.text().next())
        .map(|title| title.trim().to_string())
        .unwrap_or_else(|| "Unknown Title".to_string())
}

fn extract_description(html: &str) -> String {
    if html.is_empty() {
        return "No description available".to_string();
    }

    let document = scraper::Html::parse_document(html);
    
    let meta_selector = scraper::Selector::parse("meta[name='description'], meta[property='og:description']").unwrap();
    let description = document
        .select(&meta_selector)
        .next()
        .and_then(|element| element.value().attr("content"))
        .map(|content| content.trim().to_string());

    if let Some(desc) = description {
        if !desc.is_empty() {
            return desc;
        }
    }

    let p_selector = scraper::Selector::parse("p").unwrap();
    document
        .select(&p_selector)
        .next()
        .and_then(|element| element.text().next())
        .map(|text| text.trim().to_string())
        .unwrap_or_else(|| "No description available".to_string())
}
