use scraper::{Html, Selector};

// Convert HTML to markdown
pub fn convert_to_markdown(html: &str) -> String {
    // Basic conversion using html2md
    let mut markdown = html2md::parse_html(html);

    // Additional processing to improve markdown quality
    markdown = clean_markdown(&markdown);

    markdown
}

// Clean up the markdown for better readability
fn clean_markdown(markdown: &str) -> String {
    let mut result = markdown.to_string();

    // Remove excessive blank lines
    while result.contains("\n\n\n") {
        result = result.replace("\n\n\n", "\n\n");
    }

    // Fix code blocks
    result = result.replace("```\n```", "");

    // Fix links
    let lines: Vec<&str> = result.lines().collect();
    let mut cleaned_lines = Vec::new();

    for line in lines {
        let trimmed = line.trim();

        // Skip empty lines if the previous line was also empty
        if trimmed.is_empty()
            && cleaned_lines
                .last()
                .map_or(false, |l: &String| l.trim().is_empty())
        {
            continue;
        }

        cleaned_lines.push(line.to_string());
    }

    cleaned_lines.join("\n")
}

// Extract page title from HTML
pub fn extract_title(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    let title_selector = Selector::parse("title").ok()?;

    document
        .select(&title_selector)
        .next()
        .map(|element| element.text().collect::<Vec<_>>().join(""))
}

// Extract metadata from HTML
pub fn extract_metadata(html: &str) -> Vec<(String, String)> {
    let document = Html::parse_document(html);
    let meta_selector = Selector::parse("meta[name][content]").unwrap();

    document
        .select(&meta_selector)
        .filter_map(|element| {
            let name = element.value().attr("name")?;
            let content = element.value().attr("content")?;
            Some((name.to_string(), content.to_string()))
        })
        .collect()
}
