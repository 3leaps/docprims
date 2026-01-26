//! DOCX (Word) text extraction.

use crate::common::{open_archive, read_archive_file, resolve_entity};
use docprims_core::{DocprimsError, ExtractedText, Result};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::io::{Read, Seek};

/// Main document path in DOCX archive.
const DOCUMENT_PATH: &str = "word/document.xml";

/// Extract text from a DOCX reader.
pub fn extract<R: Read + Seek>(reader: R) -> Result<ExtractedText> {
    let mut archive = open_archive(reader)?;

    // Read main document
    let document_xml = read_archive_file(&mut archive, DOCUMENT_PATH)?
        .ok_or_else(|| DocprimsError::Malformed("Missing word/document.xml".to_string()))?;

    let text = extract_text_from_document(&document_xml)?;

    Ok(ExtractedText::complete(text))
}

/// Extract text from document.xml content.
fn extract_text_from_document(xml: &str) -> Result<String> {
    let mut reader = Reader::from_str(xml);
    // Don't use trim_text - it drops entity references in quick-xml 0.39

    let mut text = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local_name = e.local_name();
                match local_name.as_ref() {
                    b"p" => {
                        // Paragraph start - add newline if we have content
                        if !text.is_empty() && !text.ends_with('\n') {
                            text.push('\n');
                        }
                    }
                    b"br" => {
                        // Line break
                        text.push('\n');
                    }
                    b"tab" => {
                        // Tab character
                        text.push('\t');
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                let decoded = e
                    .decode()
                    .map_err(|e| DocprimsError::Parse(format!("XML decode error: {}", e)))?;
                text.push_str(&decoded);
            }
            Ok(Event::GeneralRef(e)) => {
                if let Some(resolved) = resolve_entity(&e) {
                    text.push_str(resolved);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(DocprimsError::Parse(format!("DOCX parse error: {}", e)));
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_simple_document() {
        // Minimal document.xml content
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r>
        <w:t>Hello, World!</w:t>
      </w:r>
    </w:p>
  </w:body>
</w:document>"#;

        let result = extract_text_from_document(xml).unwrap();
        assert!(result.contains("Hello, World!"));
    }

    #[test]
    fn test_multiple_paragraphs() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>First paragraph.</w:t></w:r></w:p>
    <w:p><w:r><w:t>Second paragraph.</w:t></w:r></w:p>
  </w:body>
</w:document>"#;

        let result = extract_text_from_document(xml).unwrap();
        assert!(result.contains("First paragraph."));
        assert!(result.contains("Second paragraph."));
    }
}
