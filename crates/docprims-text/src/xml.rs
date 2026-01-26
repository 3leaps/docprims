//! XML text extraction.

use docprims_core::{
    DocprimsBlock, DocprimsByteRange, DocprimsDocument, DocprimsError, DocprimsExtract,
    DocprimsGenerator, DocprimsQuality, DocprimsSource, ExtractLimits, ExtractedText, Result,
};
use quick_xml::escape::resolve_xml_entity;
use quick_xml::events::Event;
use quick_xml::reader::Reader;

/// Extract plain text from XML content.
///
/// Extracts all text nodes from the XML document, joining them with spaces.
pub fn extract(content: &str) -> Result<ExtractedText> {
    let mut reader = Reader::from_str(content);
    // Don't use trim_text - it drops entity references in quick-xml 0.39

    let mut text_parts: Vec<String> = Vec::new();
    let mut current_part = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Text(e)) => {
                let decoded = e
                    .decode()
                    .map_err(|e| DocprimsError::Parse(format!("XML decode error: {}", e)))?;
                current_part.push_str(&decoded);
            }
            Ok(Event::GeneralRef(e)) => {
                // Resolve entity reference (e.g., "lt" -> "<", "amp" -> "&")
                let entity_name = std::str::from_utf8(&e)
                    .map_err(|e| DocprimsError::Parse(format!("Invalid entity encoding: {}", e)))?;

                if let Some(resolved) = resolve_xml_entity(entity_name) {
                    current_part.push_str(resolved);
                } else if entity_name.starts_with('#') {
                    // Numeric character reference (&#NNN; or &#xHHH;)
                    if let Some(ch) = parse_numeric_entity(entity_name) {
                        current_part.push(ch);
                    }
                }
                // Unknown entities are silently dropped
            }
            Ok(Event::CData(e)) => {
                let cdata_content = String::from_utf8_lossy(&e);
                current_part.push_str(&cdata_content);
            }
            Ok(Event::Start(_) | Event::End(_)) => {
                // Element boundaries - flush current text part
                let trimmed = current_part.trim();
                if !trimmed.is_empty() {
                    text_parts.push(trimmed.to_string());
                }
                current_part.clear();
            }
            Ok(Event::Eof) => {
                // Flush any remaining text
                let trimmed = current_part.trim();
                if !trimmed.is_empty() {
                    text_parts.push(trimmed.to_string());
                }
                break;
            }
            Err(e) => {
                return Err(DocprimsError::Parse(format!(
                    "XML parse error at position {}: {}",
                    reader.buffer_position(),
                    e
                )));
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(ExtractedText::complete(text_parts.join(" ")))
}

/// Extract structured output (v0 contract) from XML content.
pub fn extract_v0_str(
    content: &str,
    source_uri: &str,
    limits: ExtractLimits,
) -> Result<DocprimsExtract> {
    let mut reader = Reader::from_str(content);
    // Don't use trim_text - it drops entity references in quick-xml 0.39

    let mut raw_parts: Vec<String> = Vec::new();
    let mut current_part = String::new();
    let mut buf = Vec::new();
    let mut hit_max_blocks = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Text(e)) => {
                let decoded = e
                    .decode()
                    .map_err(|e| DocprimsError::Parse(format!("XML decode error: {}", e)))?;
                current_part.push_str(&decoded);
            }
            Ok(Event::GeneralRef(e)) => {
                let entity_name = std::str::from_utf8(&e)
                    .map_err(|e| DocprimsError::Parse(format!("Invalid entity encoding: {}", e)))?;

                if let Some(resolved) = resolve_xml_entity(entity_name) {
                    current_part.push_str(resolved);
                } else if entity_name.starts_with('#') {
                    if let Some(ch) = parse_numeric_entity(entity_name) {
                        current_part.push(ch);
                    }
                }
            }
            Ok(Event::CData(e)) => {
                let cdata_content = String::from_utf8_lossy(&e);
                current_part.push_str(&cdata_content);
            }
            Ok(Event::Start(_) | Event::End(_)) => {
                let trimmed = current_part.trim();
                if !trimmed.is_empty() {
                    if raw_parts.len() < limits.max_blocks {
                        raw_parts.push(trimmed.to_string());
                    } else {
                        hit_max_blocks = true;
                        break;
                    }
                }
                current_part.clear();
            }
            Ok(Event::Eof) => {
                let trimmed = current_part.trim();
                if !trimmed.is_empty() {
                    if raw_parts.len() < limits.max_blocks {
                        raw_parts.push(trimmed.to_string());
                    } else {
                        hit_max_blocks = true;
                    }
                }
                break;
            }
            Err(e) => {
                return Err(DocprimsError::Parse(format!(
                    "XML parse error at position {}: {}",
                    reader.buffer_position(),
                    e
                )));
            }
            _ => {}
        }
        buf.clear();
    }

    let mut warnings = Vec::new();
    let mut quality = DocprimsQuality::complete();
    if hit_max_blocks {
        quality = DocprimsQuality::partial(docprims_core::DOCPRIMS_V0_PARTIAL_MAX_BLOCKS);
        warnings.push(docprims_core::DOCPRIMS_V0_WARN_TRUNCATED_MAX_BLOCKS.to_string());
    }

    let mut blocks = Vec::with_capacity(raw_parts.len());
    let mut doc_text = String::new();
    let mut truncated_output = false;

    for (i, text) in raw_parts.into_iter().enumerate() {
        if i > 0 {
            if doc_text.len() + 1 > limits.max_output_bytes {
                truncated_output = true;
                break;
            }
            doc_text.push(' ');
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

        blocks.push(DocprimsBlock {
            id: format!("xml:text:{}", i),
            kind: "xml:text".to_string(),
            text,
            doc_text_range: DocprimsByteRange {
                start_byte: start,
                end_byte: end,
            },
            loc: docprims_core::DocprimsLocation::file("xml:locator", source_uri)
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

    Ok(DocprimsExtract::v0(
        DocprimsGenerator {
            name: "docprims".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        DocprimsSource {
            uri: source_uri.to_string(),
            format: docprims_core::DocprimsFormat {
                family: "text".to_string(),
                kind: "xml".to_string(),
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

/// Parse a numeric character reference like "#65" or "#x41" to a char.
fn parse_numeric_entity(entity: &str) -> Option<char> {
    let s = entity.strip_prefix('#')?;
    let code = if let Some(hex) = s.strip_prefix('x').or_else(|| s.strip_prefix('X')) {
        u32::from_str_radix(hex, 16).ok()?
    } else {
        s.parse::<u32>().ok()?
    };
    char::from_u32(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_xml() {
        let xml = "<root><item>Hello</item><item>World</item></root>";
        let result = extract(xml).unwrap();
        assert!(result.content.contains("Hello"));
        assert!(result.content.contains("World"));
    }

    #[test]
    fn test_nested_xml() {
        let xml = "<root><parent><child>Text content</child></parent></root>";
        let result = extract(xml).unwrap();
        assert_eq!(result.content, "Text content");
    }

    #[test]
    fn test_cdata() {
        let xml = "<root><![CDATA[Some CDATA content]]></root>";
        let result = extract(xml).unwrap();
        assert!(result.content.contains("Some CDATA content"));
    }

    #[test]
    fn test_entities() {
        let xml = "<root>&lt;tag&gt; &amp; stuff</root>";
        let result = extract(xml).unwrap();
        assert!(result.content.contains("<tag>"));
        assert!(result.content.contains("&"));
    }

    #[test]
    fn v0_validates_schema() {
        let xml = "<root><item>Hello</item><item>World</item></root>";
        let limits = ExtractLimits {
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            max_blocks: 100,
        };
        let out = extract_v0_str(xml, "./x.xml", limits).unwrap();
        assert!(out.document.text.contains("Hello"));
        assert!(out.document.text.contains("World"));
        crate::test_support::assert_v0_schema_valid(&out);
    }
}
