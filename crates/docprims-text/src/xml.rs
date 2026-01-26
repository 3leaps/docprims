//! XML text extraction.

use docprims_core::{DocprimsError, ExtractedText, Result};
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
}
