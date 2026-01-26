//! PPTX (PowerPoint) text extraction.

use crate::common::{open_archive, read_archive_file, resolve_entity};
use docprims_core::{DocprimsError, ExtractedText, Result};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::collections::HashMap;
use std::io::{Read, Seek};

/// Presentation path in PPTX archive.
const PRESENTATION_PATH: &str = "ppt/presentation.xml";

/// Presentation relationships path.
const PRESENTATION_RELS_PATH: &str = "ppt/_rels/presentation.xml.rels";

/// Extract text from a PPTX reader.
pub fn extract<R: Read + Seek>(reader: R) -> Result<ExtractedText> {
    let mut archive = open_archive(reader)?;

    let mut all_text = String::new();

    let slide_paths = enumerate_slides(&mut archive)?;
    for (i, slide_path) in slide_paths.iter().enumerate() {
        if let Some(xml) = read_archive_file(&mut archive, slide_path)? {
            if i > 0 {
                all_text.push_str("\n\n---\n\n");
            }
            let slide_text = extract_slide_text(&xml)?;
            all_text.push_str(&slide_text);
        }
    }

    Ok(ExtractedText::complete(all_text.trim().to_string()))
}

fn enumerate_slides<R: Read + Seek>(archive: &mut zip::ZipArchive<R>) -> Result<Vec<String>> {
    let pres_xml = read_archive_file(archive, PRESENTATION_PATH)?
        .ok_or_else(|| DocprimsError::Malformed("Missing ppt/presentation.xml".to_string()))?;
    let rels_xml = read_archive_file(archive, PRESENTATION_RELS_PATH)?.ok_or_else(|| {
        DocprimsError::Malformed("Missing ppt/_rels/presentation.xml.rels".to_string())
    })?;

    let slide_rids = parse_presentation_slide_rids(&pres_xml)?;
    let rels = parse_relationships(&rels_xml)?;

    let mut out = Vec::new();
    for rid in slide_rids {
        if let Some(target) = rels.get(&rid) {
            out.push(resolve_ooxml_target("ppt", target));
        }
    }
    Ok(out)
}

fn parse_presentation_slide_rids(xml: &str) -> Result<Vec<String>> {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut rids = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(e)) | Ok(Event::Start(e)) => {
                if e.local_name().as_ref() == b"sldId" {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"r:id" {
                            rids.push(String::from_utf8_lossy(&attr.value).to_string());
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(DocprimsError::Parse(format!(
                    "presentation.xml parse error at {}: {}",
                    reader.buffer_position(),
                    e
                )));
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(rids)
}

fn parse_relationships(xml: &str) -> Result<HashMap<String, String>> {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut rels = HashMap::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(e)) | Ok(Event::Start(e)) => {
                if e.local_name().as_ref() == b"Relationship" {
                    let mut id: Option<String> = None;
                    let mut target: Option<String> = None;
                    for attr in e.attributes().flatten() {
                        match attr.key.as_ref() {
                            b"Id" => id = Some(String::from_utf8_lossy(&attr.value).to_string()),
                            b"Target" => {
                                target = Some(String::from_utf8_lossy(&attr.value).to_string())
                            }
                            _ => {}
                        }
                    }
                    if let (Some(id), Some(target)) = (id, target) {
                        rels.insert(id, target);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(DocprimsError::Parse(format!(
                    "rels parse error at {}: {}",
                    reader.buffer_position(),
                    e
                )));
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(rels)
}

fn resolve_ooxml_target(base_dir: &str, target: &str) -> String {
    let target = target.trim();
    let target = target.strip_prefix('/').unwrap_or(target);

    let mut parts: Vec<&str> = base_dir.split('/').filter(|s| !s.is_empty()).collect();
    for seg in target.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if !parts.is_empty() {
                    parts.pop();
                }
            }
            _ => parts.push(seg),
        }
    }

    parts.join("/")
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

    #[test]
    fn test_enumerate_slides_from_presentation_and_rels() {
        let pres = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:sldIdLst>
    <p:sldId id="256" r:id="rId2"/>
    <p:sldId id="257" r:id="rId5"/>
  </p:sldIdLst>
</p:presentation>"#;

        let rels = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Target="slides/slide1.xml"/>
  <Relationship Id="rId5" Target="slides/slide2.xml"/>
</Relationships>"#;

        let ids = parse_presentation_slide_rids(pres).unwrap();
        assert_eq!(ids, vec!["rId2".to_string(), "rId5".to_string()]);

        let map = parse_relationships(rels).unwrap();
        let p1 = resolve_ooxml_target("ppt", map.get("rId2").unwrap());
        let p2 = resolve_ooxml_target("ppt", map.get("rId5").unwrap());
        assert_eq!(p1, "ppt/slides/slide1.xml");
        assert_eq!(p2, "ppt/slides/slide2.xml");
    }
}
