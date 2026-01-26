//! PPTX (PowerPoint) text extraction.

use crate::common::{open_archive, read_archive_file, resolve_entity};
use docprims_core::{DocprimsError, ExtractedText, Result};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::io::{Read, Seek};

/// Extract text from a PPTX reader.
pub fn extract<R: Read + Seek>(reader: R) -> Result<ExtractedText> {
    let mut archive = open_archive(reader)?;

    let mut all_text = String::new();

    // Extract text from slides
    // TODO: Enumerate slides from presentation.xml
    for i in 1..=100 {
        let slide_path = format!("ppt/slides/slide{}.xml", i);
        match read_archive_file(&mut archive, &slide_path)? {
            Some(xml) => {
                if !all_text.is_empty() {
                    all_text.push_str("\n\n---\n\n"); // Slide separator
                }
                let slide_text = extract_slide_text(&xml)?;
                all_text.push_str(&slide_text);
            }
            None => break, // No more slides
        }
    }

    Ok(ExtractedText::complete(all_text.trim().to_string()))
}

/// Extract text from a slide XML.
fn extract_slide_text(xml: &str) -> Result<String> {
    let mut reader = Reader::from_str(xml);
    // Don't use trim_text - it drops entity references in quick-xml 0.39

    let mut text_parts: Vec<String> = Vec::new();
    let mut current_part = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local_name = e.local_name();
                if local_name.as_ref() == b"p" {
                    // Paragraph start - flush and prepare for new paragraph
                    let trimmed = current_part.trim();
                    if !trimmed.is_empty() {
                        text_parts.push(trimmed.to_string());
                    }
                    current_part.clear();
                }
            }
            Ok(Event::End(e)) => {
                let local_name = e.local_name();
                if local_name.as_ref() == b"p" {
                    // Paragraph end - flush current content
                    let trimmed = current_part.trim();
                    if !trimmed.is_empty() {
                        text_parts.push(trimmed.to_string());
                    }
                    current_part.clear();
                }
            }
            Ok(Event::Text(e)) => {
                let decoded = e
                    .decode()
                    .map_err(|e| DocprimsError::Parse(format!("XML decode error: {}", e)))?;
                current_part.push_str(&decoded);
            }
            Ok(Event::GeneralRef(e)) => {
                if let Some(resolved) = resolve_entity(&e) {
                    current_part.push_str(resolved);
                }
            }
            Ok(Event::Eof) => {
                let trimmed = current_part.trim();
                if !trimmed.is_empty() {
                    text_parts.push(trimmed.to_string());
                }
                break;
            }
            Err(e) => {
                return Err(DocprimsError::Parse(format!("Slide parse error: {}", e)));
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(text_parts.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_slide_text() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:sp>
        <p:txBody>
          <a:p>
            <a:r>
              <a:t>Slide Title</a:t>
            </a:r>
          </a:p>
          <a:p>
            <a:r>
              <a:t>Bullet point text</a:t>
            </a:r>
          </a:p>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

        let result = extract_slide_text(xml).unwrap();
        assert!(result.contains("Slide Title"));
        assert!(result.contains("Bullet point text"));
    }
}
