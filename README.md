# Site-Vacuum

A web scraper that converts websites to Markdown, with support for client-side rendering.

## Features

- Crawls websites and converts them to Markdown
- Supports client-side rendered applications (React, Angular, Vue, etc.) using a completely headless browser
- Detects when JavaScript rendering is needed
- Concurrency control for faster crawling
- Server-Sent Events (SSE) for real-time crawler status

## Installation

```bash
cargo build --release
```

## Dependencies

For JavaScript rendering support, you need to have ChromeDriver installed and running:

1. Install ChromeDriver (matches your Chrome version): 
   - Download from https://chromedriver.chromium.org/downloads
   - Or use your package manager

2. Run ChromeDriver before using Site-Vacuum with JS rendering:
   ```bash
   chromedriver --port=9515
   ```

## Usage

### Starting the Server

```bash
cargo run --release
```

The server will start at http://127.0.0.1:8000

### API Endpoints

#### POST /crawl

Crawl a website and get Markdown content.

Example request:

```json
{
  "url": "https://example.com",
  "concurrent": 5,
  "enable_js_rendering": true
}
```

Parameters:
- `url`: The website URL to crawl
- `concurrent`: Number of concurrent requests (recommended: 5-10)
- `enable_js_rendering`: Set to `true` to handle client-side rendered apps

Example response:

```json
{
  "markdown": "# Example Website\n\nThis is the content..."
}
```

## How JavaScript Rendering Works

When `enable_js_rendering` is enabled or automatically detected:

1. The crawler launches a completely headless Chrome browser via WebDriver (no visible window)
2. Navigates to the target URL and waits for JavaScript to execute
3. Captures the fully rendered HTML after JavaScript has loaded
4. Converts the rendered HTML to Markdown
5. Automatically closes the browser session

This ensures that client-side rendered applications (like React, Angular, Vue) are properly scraped with all dynamic content without any visible browser windows opening on your desktop.

## Automatic Detection

Even without explicitly enabling JavaScript rendering, Site-Vacuum will try to detect client-side rendered apps by looking for common JavaScript framework patterns and automatically use the headless browser when needed.

## Server-Sent Events (SSE) Endpoint

The application includes an SSE endpoint that allows clients to receive real-time updates on the sites the crawler is visiting.

### Endpoint

```
GET /events
```

### Event Format

Events are sent as JSON with the following structure:

```json
{
  "event_type": "VisitingUrl",
  "url": "https://example.com/page",
  "message": "Processing URL: https://example.com/page",
  "timestamp": 1621234567
}
```

### Event Types

- `Started`: Crawl has started
- `VisitingUrl`: Crawler is about to process a URL
- `UrlProcessed`: Crawler has processed a URL
- `Error`: An error occurred while processing a URL
- `Completed`: Crawl has completed

## Usage Example

A demo HTML client is provided in `examples/sse-client.html`. Open this file in a browser to see a live dashboard of crawler status.

### JavaScript Client Example

```javascript
// Connect to the SSE endpoint
const eventSource = new EventSource('http://localhost:8000/events');

// Handle incoming events
eventSource.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log(`Event: ${data.event_type} for ${data.url}`);
  console.log(`Message: ${data.message}`);
};
```

## API Endpoints

- `GET /` - Health check endpoint
- `POST /crawl` - Start a new crawl and return markdown
- `GET /events` - SSE endpoint for crawler updates

## Setup

```bash
# Run the server
cargo run

# For JS rendering support, make sure you have chromedriver running:
chromedriver --port=9515
``` 