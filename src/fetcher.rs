use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;
use url::Url;
use thirtyfour::{WebDriver, DesiredCapabilities};
use tracing::info;

pub async fn fetch_url(url: &Url) -> Result<String> {
    info!("Fetching URL (regular): {}", url);
    
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("site-vacuum/0.1 (https://github.com/yourusername/site-vacuum)")
        .build()
        .context("Failed to build HTTP client")?;

    let response = client
        .get(url.as_str())
        .send()
        .await
        .with_context(|| format!("Failed to fetch URL: {}", url))?;

    if !response.status().is_success() {
        anyhow::bail!(
            "HTTP error when fetching {}: {} ({})",
            url,
            response.status().as_u16(),
            response.status().canonical_reason().unwrap_or("Unknown")
        );
    }

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Only process HTML content
    if !content_type.contains("text/html") {
        anyhow::bail!("Non-HTML content type: {}", content_type);
    }

    let text = response
        .text()
        .await
        .context("Failed to read response body")?;

    Ok(text)
}

/// Fetches a URL's content after rendering JavaScript using a headless browser
pub async fn fetch_url_with_js(url: &Url) -> Result<String> {
    info!("Fetching URL (with JS rendering): {}", url);
    
    // Create WebDriver capabilities with headless mode
    let mut caps = DesiredCapabilities::chrome();
    
    // Configure Chrome to run in headless mode
    caps.add_chrome_arg("--headless")?;
    caps.add_chrome_arg("--disable-gpu")?;
    caps.add_chrome_arg("--no-sandbox")?;
    caps.add_chrome_arg("--disable-dev-shm-usage")?;
    
    // Connect to WebDriver instance
    let driver = WebDriver::new("http://localhost:9515", caps)
        .await
        .context("Failed to connect to WebDriver. Is chromedriver running?")?;
    
    // Navigate to the URL
    driver.goto(url.as_str())
        .await
        .with_context(|| format!("Failed to navigate to URL: {}", url))?;
    
    // Wait for the page to load (wait for the document to be ready)
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Get the page source after JavaScript execution
    let html = driver.source()
        .await
        .context("Failed to get page source")?;
    
    // Close the browser
    driver.quit().await?;
    
    Ok(html)
}

pub async fn smart_fetch_url(url: &Url, force_js_rendering: bool) -> Result<String> {
    info!("Smart fetching URL: {}", url);
    
    if force_js_rendering {
        fetch_url_with_js(url).await
    } else {
        match fetch_url(url).await {
            Ok(html) => {
                // Simple heuristic: Check if the page likely requires JS rendering
                if html.contains("window.addEventListener") || 
                   html.contains("document.getElementById") ||
                   html.contains("ReactDOM") ||
                   html.contains("ng-app") ||
                   html.contains("v-app") {
                    // Likely a client-side rendered app, retry with JS rendering
                    info!("Detected client-side rendering for URL: {}", url);
                    fetch_url_with_js(url).await
                } else {
                    Ok(html)
                }
            },
            Err(_e) => {
                // If regular fetch fails, try with JS rendering as fallback
                info!("Regular fetch failed, trying with JS rendering for URL: {}", url);
                fetch_url_with_js(url).await
            }
        }
    }
}
