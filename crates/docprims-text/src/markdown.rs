//! Markdown text extraction.

use docprims_core::{
    DocprimsBlock, DocprimsByteRange, DocprimsDocument, DocprimsExtract, DocprimsGenerator,
    DocprimsQuality, DocprimsSource, ExtractLimits, ExtractedText, Result,
};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};

fn normalize_newlines(s: &str) -> String {
    // Normalize CRLF/CR to LF for deterministic offsets.
    s.replace("\r\n", "\n").replace('\r', "\n")
}

/// Extract plain text from Markdown content.
pub fn extract(content: &str) -> Result<ExtractedText> {
    let parser = Parser::new(content);
    let mut text = String::new();

    for event in parser {
        match event {
            Event::Text(t) => {
                text.push_str(&t);
            }
            Event::Code(c) => {
                text.push_str(&c);
            }
            Event::SoftBreak | Event::HardBreak => {
                text.push('\n');
            }
            Event::Start(Tag::CodeBlock(_)) => {}
            Event::End(TagEnd::CodeBlock) => {
                text.push('\n');
            }
            Event::Start(Tag::Paragraph) => {
                if !text.is_empty() && !text.ends_with('\n') {
                    text.push('\n');
                }
            }
            Event::End(TagEnd::Paragraph) => {
                text.push('\n');
            }
            Event::Start(Tag::Heading { .. }) => {
                if !text.is_empty() && !text.ends_with('\n') {
                    text.push('\n');
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                text.push('\n');
            }
            Event::Start(Tag::Item) => {
                if !text.is_empty() && !text.ends_with('\n') {
                    text.push('\n');
                }
            }
            Event::End(TagEnd::Item) => {
                if !text.ends_with('\n') {
                    text.push('\n');
                }
            }
            _ => {}
        }
    }

    // Trim trailing whitespace but preserve structure
    let content = text.trim_end().to_string();

    Ok(ExtractedText::complete(content))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MdBlockKind {
    Heading,
    Paragraph,
    ListItem,
    Code,
}

impl MdBlockKind {
    fn as_kind_str(self) -> &'static str {
        match self {
            MdBlockKind::Heading => "markdown:heading",
            MdBlockKind::Paragraph => "markdown:paragraph",
            MdBlockKind::ListItem => "markdown:list_item",
            MdBlockKind::Code => "markdown:code",
        }
    }

    fn id_prefix(self) -> &'static str {
        match self {
            MdBlockKind::Heading => "markdown:heading",
            MdBlockKind::Paragraph => "markdown:paragraph",
            MdBlockKind::ListItem => "markdown:list_item",
            MdBlockKind::Code => "markdown:code",
        }
    }
}

/// Extract structured output (v0 contract) from Markdown content.
pub fn extract_v0_str(
    content: &str,
    source_uri: &str,
    limits: ExtractLimits,
) -> Result<DocprimsExtract> {
    let content = normalize_newlines(content);
    let parser = Parser::new(&content);

    let mut raw_blocks: Vec<(MdBlockKind, String)> = Vec::new();
    let mut cur_kind: Option<MdBlockKind> = None;
    let mut cur_text = String::new();
    let mut in_item = false;

    let flush = |raw_blocks: &mut Vec<(MdBlockKind, String)>,
                 cur_kind: &mut Option<MdBlockKind>,
                 cur_text: &mut String| {
        if let Some(kind) = cur_kind.take() {
            let t = cur_text.trim_end().to_string();
            cur_text.clear();
            if !t.is_empty() {
                raw_blocks.push((kind, t));
            }
        } else {
            cur_text.clear();
        }
    };

    let ensure_block = |kind: MdBlockKind,
                        raw_blocks: &mut Vec<(MdBlockKind, String)>,
                        cur_kind: &mut Option<MdBlockKind>,
                        cur_text: &mut String| {
        if cur_kind.is_some() {
            flush(raw_blocks, cur_kind, cur_text);
        }
        *cur_kind = Some(kind);
    };

    for event in parser {
        match event {
            Event::Start(Tag::Heading { .. }) => {
                ensure_block(
                    MdBlockKind::Heading,
                    &mut raw_blocks,
                    &mut cur_kind,
                    &mut cur_text,
                );
            }
            Event::End(TagEnd::Heading(_)) => {
                flush(&mut raw_blocks, &mut cur_kind, &mut cur_text);
            }
            Event::Start(Tag::Item) => {
                in_item = true;
                ensure_block(
                    MdBlockKind::ListItem,
                    &mut raw_blocks,
                    &mut cur_kind,
                    &mut cur_text,
                );
            }
            Event::End(TagEnd::Item) => {
                in_item = false;
                flush(&mut raw_blocks, &mut cur_kind, &mut cur_text);
            }
            Event::Start(Tag::Paragraph) => {
                if in_item {
                    if !cur_text.is_empty() && !cur_text.ends_with('\n') {
                        cur_text.push('\n');
                    }
                } else {
                    ensure_block(
                        MdBlockKind::Paragraph,
                        &mut raw_blocks,
                        &mut cur_kind,
                        &mut cur_text,
                    );
                }
            }
            Event::End(TagEnd::Paragraph) => {
                if in_item {
                    if !cur_text.ends_with('\n') {
                        cur_text.push('\n');
                    }
                } else {
                    flush(&mut raw_blocks, &mut cur_kind, &mut cur_text);
                }
            }
            Event::Start(Tag::CodeBlock(_)) => {
                ensure_block(
                    MdBlockKind::Code,
                    &mut raw_blocks,
                    &mut cur_kind,
                    &mut cur_text,
                );
            }
            Event::End(TagEnd::CodeBlock) => {
                flush(&mut raw_blocks, &mut cur_kind, &mut cur_text);
            }
            Event::Text(t) => {
                if cur_kind.is_none() {
                    ensure_block(
                        MdBlockKind::Paragraph,
                        &mut raw_blocks,
                        &mut cur_kind,
                        &mut cur_text,
                    );
                }
                cur_text.push_str(&t);
            }
            Event::Code(c) => {
                if cur_kind.is_none() {
                    ensure_block(
                        MdBlockKind::Paragraph,
                        &mut raw_blocks,
                        &mut cur_kind,
                        &mut cur_text,
                    );
                }
                cur_text.push_str(&c);
            }
            Event::SoftBreak | Event::HardBreak => {
                if cur_kind.is_none() {
                    ensure_block(
                        MdBlockKind::Paragraph,
                        &mut raw_blocks,
                        &mut cur_kind,
                        &mut cur_text,
                    );
                }
                cur_text.push('\n');
            }
            _ => {}
        }

        if raw_blocks.len() >= limits.max_blocks {
            break;
        }
    }
    flush(&mut raw_blocks, &mut cur_kind, &mut cur_text);

    let mut warnings = Vec::new();
    let mut quality = DocprimsQuality::complete();

    if raw_blocks.len() >= limits.max_blocks {
        quality = DocprimsQuality::partial(docprims_core::DOCPRIMS_V0_PARTIAL_MAX_BLOCKS);
        warnings.push(docprims_core::DOCPRIMS_V0_WARN_TRUNCATED_MAX_BLOCKS.to_string());
        raw_blocks.truncate(limits.max_blocks);
    }

    let mut blocks = Vec::with_capacity(raw_blocks.len());
    let mut doc_text = String::new();
    let mut truncated_output = false;

    for (i, (kind, text)) in raw_blocks.into_iter().enumerate() {
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
            docprims_core::truncate_to_utf8_boundary(&text, remaining).to_string()
        } else {
            text
        };

        doc_text.push_str(&text);
        let end = doc_text.len();

        blocks.push(DocprimsBlock {
            id: format!("{}:{}", kind.id_prefix(), i),
            kind: kind.as_kind_str().to_string(),
            text,
            doc_text_range: DocprimsByteRange {
                start_byte: start,
                end_byte: end,
            },
            loc: docprims_core::DocprimsLocation::file("markdown:locator", source_uri)
                .with_hint_u64("block_index", i as u64),
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

    let document = DocprimsDocument {
        quality,
        text: doc_text,
        blocks,
        warnings,
        metadata: None,
    };

    Ok(DocprimsExtract::v0(
        DocprimsGenerator {
            name: "docprims".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        DocprimsSource {
            uri: source_uri.to_string(),
            format: docprims_core::DocprimsFormat {
                family: "text".to_string(),
                kind: "markdown".to_string(),
            },
            sha256: None,
        },
        document,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_markdown() {
        let md = "# Hello\n\nThis is a paragraph.";
        let result = extract(md).unwrap();
        assert!(result.content.contains("Hello"));
        assert!(result.content.contains("This is a paragraph"));
    }

    #[test]
    fn test_code_block() {
        let md = "```rust\nfn main() {}\n```";
        let result = extract(md).unwrap();
        assert!(result.content.contains("fn main()"));
    }

    #[test]
    fn v0_blocks_have_offsets() {
        let md = "# H\n\n- a\n- b\n\npara";
        let limits = ExtractLimits {
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            max_blocks: 100,
        };
        let out = extract_v0_str(md, "./x.md", limits).unwrap();
        assert!(!out.document.blocks.is_empty());
        for b in &out.document.blocks {
            assert!(b.doc_text_range.end_byte >= b.doc_text_range.start_byte);
            assert!(b.doc_text_range.end_byte <= out.document.text.len());
        }
    }

    #[test]
    fn v0_is_deterministic() {
        let md = "# H\n\npara\n\n- a\n- b";
        let limits = ExtractLimits {
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            max_blocks: 100,
        };

        let a = extract_v0_str(md, "./x.md", limits).unwrap();
        let b = extract_v0_str(md, "./x.md", limits).unwrap();
        let a_json = serde_json::to_string(&a).unwrap();
        let b_json = serde_json::to_string(&b).unwrap();
        assert_eq!(a_json, b_json);
    }

    #[test]
    fn v0_byte_ranges_are_utf8_bytes() {
        // 'é' is 2 bytes in UTF-8; this test ensures ranges are byte offsets.
        let md = "café";
        let limits = ExtractLimits {
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            max_blocks: 10,
        };

        let out = extract_v0_str(md, "./x.md", limits).unwrap();
        let b0 = &out.document.blocks[0];
        let len = b0.doc_text_range.end_byte - b0.doc_text_range.start_byte;
        assert_eq!(len, "café".len());
        assert_eq!(b0.text.len(), "café".len());
    }

    #[test]
    fn v0_validates_against_local_schema() {
        let md = "# H\n\npara";
        let limits = ExtractLimits {
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            max_blocks: 100,
        };
        let out = extract_v0_str(md, "./x.md", limits).unwrap();
        crate::test_support::assert_v0_schema_valid(&out);
    }

    #[test]
    fn v0_truncation_uses_stable_warning_and_reason_constants() {
        let md = "# H\n\npara\n\n- a\n- b";
        let limits = ExtractLimits {
            max_input_bytes: 1024,
            max_output_bytes: 5,
            max_blocks: 100,
        };

        let out = extract_v0_str(md, "./x.md", limits).unwrap();
        assert!(matches!(
            out.document.quality.reason.as_deref(),
            Some(docprims_core::DOCPRIMS_V0_PARTIAL_MAX_OUTPUT_BYTES)
        ));
        assert!(out
            .document
            .warnings
            .iter()
            .any(|w| w == docprims_core::DOCPRIMS_V0_WARN_TRUNCATED_MAX_OUTPUT_BYTES));
    }

    #[test]
    fn v0_output_truncation_preserves_utf8_boundaries_for_ranges() {
        // The byte budget is chosen to cut within a multi-byte character if slicing is wrong.
        // We assert the resulting text and block ranges remain valid UTF-8 and consistent.
        let md = "café";
        let limits = ExtractLimits {
            max_input_bytes: 1024,
            max_output_bytes: 4,
            max_blocks: 10,
        };

        let out = extract_v0_str(md, "./x.md", limits).unwrap();
        assert!(out.document.text.is_char_boundary(out.document.text.len()));

        let b0 = &out.document.blocks[0];
        assert!(b0.text.is_char_boundary(b0.text.len()));
        assert!(b0.doc_text_range.end_byte <= out.document.text.len());
        assert_eq!(
            &out.document.text[b0.doc_text_range.start_byte..b0.doc_text_range.end_byte],
            b0.text
        );
    }
}
