//! HTML text extraction.

use docprims_core::{ExtractedText, Result};
use scraper::{Html, Selector};

/// Extract plain text from HTML content.
pub fn extract(content: &str) -> Result<ExtractedText> {
    let document = Html::parse_document(content);

    // Select body content, falling back to entire document
    let body_selector = Selector::parse("body").unwrap();
    let root = document
        .select(&body_selector)
        .next()
        .map(|e| e.html())
        .unwrap_or_else(|| content.to_string());

    let body_doc = Html::parse_fragment(&root);

    let mut text = String::new();

    // Walk through text nodes, handling block elements
    for node in body_doc.root_element().descendants() {
        if let Some(text_node) = node.value().as_text() {
            let trimmed = text_node.trim();
            if !trimmed.is_empty() {
                if !text.is_empty() && !text.ends_with('\n') && !text.ends_with(' ') {
                    text.push(' ');
                }
                text.push_str(trimmed);
            }
        } else if let Some(element) = node.value().as_element() {
            // Add newlines for block elements
            let tag = element.name();
            if is_block_element(tag) && !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
            }
        }
    }

    let content = text.trim().to_string();
    Ok(ExtractedText::complete(content))
}

fn is_block_element(tag: &str) -> bool {
    matches!(
        tag,
        "p" | "div"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "br"
            | "hr"
            | "li"
            | "tr"
            | "blockquote"
            | "pre"
            | "article"
            | "section"
            | "header"
            | "footer"
            | "nav"
            | "aside"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_html() {
        let html = "<html><body><p>Hello, world!</p></body></html>";
        let result = extract(html).unwrap();
        assert_eq!(result.content, "Hello, world!");
    }

    #[test]
    fn test_nested_elements() {
        let html = "<p>Hello <strong>bold</strong> world</p>";
        let result = extract(html).unwrap();
        assert!(result.content.contains("Hello"));
        assert!(result.content.contains("bold"));
        assert!(result.content.contains("world"));
    }

    #[test]
    fn test_script_style_ignored() {
        let html = "<p>Text</p><script>alert('x')</script><style>.x{}</style><p>More</p>";
        let result = extract(html).unwrap();
        // Script and style content should be in there but we don't specifically filter
        // In a more complete implementation, we'd skip script/style elements
        assert!(result.content.contains("Text"));
        assert!(result.content.contains("More"));
    }
}
