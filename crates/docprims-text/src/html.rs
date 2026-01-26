//! HTML text extraction.

use docprims_core::{
    DocprimsBlock, DocprimsByteRange, DocprimsDocument, DocprimsExtract, DocprimsGenerator,
    DocprimsQuality, DocprimsSource, ExtractLimits, ExtractedText, Result,
};
use scraper::{ElementRef, Html, Selector};

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

    // Walk through text nodes, handling block elements.
    // Skip script/style/noscript/template content.
    for node in body_doc.root_element().descendants() {
        if let Some(text_node) = node.value().as_text() {
            let mut excluded = false;
            for anc in node.ancestors() {
                if let Some(el) = ElementRef::wrap(anc) {
                    if is_excluded_tag(el.value().name()) {
                        excluded = true;
                        break;
                    }
                }
            }
            if excluded {
                continue;
            }

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HtmlBlockKind {
    Heading,
    Paragraph,
    ListItem,
    Code,
    Blockquote,
}

impl HtmlBlockKind {
    fn kind_str(self) -> &'static str {
        match self {
            HtmlBlockKind::Heading => "html:heading",
            HtmlBlockKind::Paragraph => "html:paragraph",
            HtmlBlockKind::ListItem => "html:list_item",
            HtmlBlockKind::Code => "html:code",
            HtmlBlockKind::Blockquote => "html:blockquote",
        }
    }

    fn id_prefix(self) -> &'static str {
        self.kind_str()
    }
}

fn is_excluded_tag(tag: &str) -> bool {
    matches!(tag, "script" | "style" | "noscript" | "template")
}

fn is_candidate_block_tag(tag: &str) -> bool {
    matches!(
        tag,
        "p" | "li" | "pre" | "blockquote" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
    )
}

fn kind_for_tag(tag: &str) -> Option<(HtmlBlockKind, Option<u64>)> {
    match tag {
        "h1" => Some((HtmlBlockKind::Heading, Some(1))),
        "h2" => Some((HtmlBlockKind::Heading, Some(2))),
        "h3" => Some((HtmlBlockKind::Heading, Some(3))),
        "h4" => Some((HtmlBlockKind::Heading, Some(4))),
        "h5" => Some((HtmlBlockKind::Heading, Some(5))),
        "h6" => Some((HtmlBlockKind::Heading, Some(6))),
        "p" => Some((HtmlBlockKind::Paragraph, None)),
        "li" => Some((HtmlBlockKind::ListItem, None)),
        "pre" => Some((HtmlBlockKind::Code, None)),
        "blockquote" => Some((HtmlBlockKind::Blockquote, None)),
        _ => None,
    }
}

fn has_excluded_ancestor(el: &ElementRef<'_>) -> bool {
    el.ancestors().any(|n| {
        ElementRef::wrap(n)
            .map(|a| is_excluded_tag(a.value().name()))
            .unwrap_or(false)
    })
}

fn should_skip_due_to_ancestor(tag: &str, el: &ElementRef<'_>) -> bool {
    if tag == "li" {
        return false;
    }

    el.ancestors().any(|n| {
        ElementRef::wrap(n)
            .map(|a| is_candidate_block_tag(a.value().name()))
            .unwrap_or(false)
    })
}

fn extract_block_text(el: &ElementRef<'_>) -> String {
    let tag = el.value().name();
    let is_pre = tag == "pre";
    let mut out = String::new();
    let root_id = el.id();

    for node in el.descendants() {
        let mut skip = false;
        let mut nested_li = false;

        for anc in node.ancestors() {
            if let Some(a) = ElementRef::wrap(anc) {
                let t = a.value().name();
                if is_excluded_tag(t) {
                    skip = true;
                    break;
                }
                if tag == "li" && t == "li" && a.id() != root_id {
                    nested_li = true;
                    break;
                }
            }
        }

        if skip || nested_li {
            continue;
        }

        if let Some(e) = ElementRef::wrap(node) {
            if e.value().name() == "br" && !out.ends_with('\n') {
                out.push('\n');
            }
            continue;
        }

        if let Some(t) = node.value().as_text() {
            if is_pre {
                out.push_str(t);
            } else {
                let trimmed = t.trim();
                if !trimmed.is_empty() {
                    if !out.is_empty() && !out.ends_with('\n') && !out.ends_with(' ') {
                        out.push(' ');
                    }
                    out.push_str(trimmed);
                }
            }
        }
    }

    if is_pre {
        out.trim_end().to_string()
    } else {
        out.trim().to_string()
    }
}

/// Extract structured output (v0 contract) from HTML content.
pub fn extract_v0_str(
    content: &str,
    source_uri: &str,
    limits: ExtractLimits,
) -> Result<DocprimsExtract> {
    let document = Html::parse_document(content);
    let body_selector = Selector::parse("body").unwrap();
    let root = document
        .select(&body_selector)
        .next()
        .unwrap_or_else(|| document.root_element());

    let blocks_selector = Selector::parse("h1,h2,h3,h4,h5,h6,p,pre,blockquote,li").unwrap();
    let mut raw_blocks: Vec<(HtmlBlockKind, Option<u64>, String, String)> = Vec::new();
    let mut hit_max_blocks = false;

    for el in root.select(&blocks_selector) {
        let tag = el.value().name();
        if has_excluded_ancestor(&el) {
            continue;
        }
        if should_skip_due_to_ancestor(tag, &el) {
            continue;
        }

        let Some((kind, level)) = kind_for_tag(tag) else {
            continue;
        };

        let text = extract_block_text(&el);
        if text.is_empty() {
            continue;
        }

        if raw_blocks.len() < limits.max_blocks {
            raw_blocks.push((kind, level, tag.to_string(), text));
        } else {
            hit_max_blocks = true;
            break;
        }
    }

    let mut warnings = Vec::new();
    let mut quality = DocprimsQuality::complete();

    if hit_max_blocks {
        quality = DocprimsQuality::partial(docprims_core::DOCPRIMS_V0_PARTIAL_MAX_BLOCKS);
        warnings.push(docprims_core::DOCPRIMS_V0_WARN_TRUNCATED_MAX_BLOCKS.to_string());
    }

    let mut blocks = Vec::with_capacity(raw_blocks.len());
    let mut doc_text = String::new();
    let mut truncated_output = false;

    for (i, (kind, level, tag, text)) in raw_blocks.into_iter().enumerate() {
        if i > 0 {
            if doc_text.len() + 1 > limits.max_output_bytes {
                truncated_output = true;
                break;
            }
            doc_text.push('\n');
        }

        let start = doc_text.len();
        let remaining = limits.max_output_bytes.saturating_sub(doc_text.len());
        let text = if text.len() > remaining {
            truncated_output = true;
            crate::truncate_to_utf8_boundary(&text, remaining).to_string()
        } else {
            text
        };
        doc_text.push_str(&text);
        let end = doc_text.len();

        let mut loc = docprims_core::DocprimsLocation::file("html:locator", source_uri)
            .with_hint_u64("block_index", i as u64)
            .with_hint_str("tag", tag);
        if let Some(level) = level {
            loc = loc.with_hint_u64("level", level);
        }

        blocks.push(DocprimsBlock {
            id: format!("{}:{}", kind.id_prefix(), i),
            kind: kind.kind_str().to_string(),
            text,
            doc_text_range: DocprimsByteRange {
                start_byte: start,
                end_byte: end,
            },
            loc,
            children: vec![],
            role: None,
        });

        if truncated_output {
            break;
        }
    }

    if truncated_output {
        quality = DocprimsQuality::partial(docprims_core::DOCPRIMS_V0_PARTIAL_MAX_OUTPUT_BYTES);
        warnings.push(docprims_core::DOCPRIMS_V0_WARN_TRUNCATED_MAX_OUTPUT_BYTES.to_string());
    }

    Ok(DocprimsExtract::v0(
        DocprimsGenerator {
            name: "docprims".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        DocprimsSource {
            uri: source_uri.to_string(),
            format: docprims_core::DocprimsFormat {
                family: "text".to_string(),
                kind: "html".to_string(),
            },
            sha256: None,
        },
        DocprimsDocument {
            quality,
            text: doc_text,
            blocks,
            warnings,
            metadata: None,
        },
    ))
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

    #[test]
    fn v0_skips_script_style_and_validates_schema() {
        let html = "<html><body><h1>Hi</h1><script>alert('x')</script><p>There</p></body></html>";
        let limits = ExtractLimits {
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            max_blocks: 100,
        };
        let out = extract_v0_str(html, "./x.html", limits).unwrap();
        assert!(out.document.text.contains("Hi"));
        assert!(out.document.text.contains("There"));
        assert!(!out.document.text.contains("alert"));
        crate::test_support::assert_v0_schema_valid(&out);
    }

    #[test]
    fn plain_skips_script_style() {
        let html = "<p>Text</p><script>alert('x')</script><style>.x{}</style><p>More</p>";
        let out = extract(html).unwrap();
        assert!(out.content.contains("Text"));
        assert!(out.content.contains("More"));
        assert!(!out.content.contains("alert"));
        assert!(!out.content.contains(".x"));
    }
}
