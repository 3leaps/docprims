//! PPTX (PowerPoint) text extraction.

use crate::common::{open_archive, read_archive_file, resolve_entity};
use docprims_core::{
    DocprimsBlock, DocprimsByteRange, DocprimsDocument, DocprimsError, DocprimsExtract,
    DocprimsGenerator, DocprimsQuality, DocprimsSource, ExtractLimits, ExtractedText, Result,
};
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

/// Extract structured output (v0 contract) from a PPTX reader.
pub fn extract_v0<R: Read + Seek>(
    reader: R,
    source_uri: &str,
    limits: ExtractLimits,
) -> Result<DocprimsExtract> {
    let mut archive = open_archive(reader)?;
    let slide_paths = enumerate_slides(&mut archive)?;

    let mut warnings = Vec::new();
    let mut quality = DocprimsQuality::complete();
    let mut blocks = Vec::new();
    let mut doc_text = String::new();

    let mut hit_max_blocks = false;
    let mut truncated_output = false;
    let mut block_index = 0usize;

    for (slide_idx0, slide_path) in slide_paths.iter().enumerate() {
        if block_index >= limits.max_blocks {
            hit_max_blocks = true;
            break;
        }

        let Some(xml) = read_archive_file(&mut archive, slide_path)? else {
            continue;
        };

        let paras = extract_slide_paragraphs(&xml)?;
        for (p_idx, p) in paras.into_iter().enumerate() {
            if p.is_empty() {
                continue;
            }

            if block_index >= limits.max_blocks {
                hit_max_blocks = true;
                break;
            }

            if block_index > 0 {
                if doc_text.len() + 1 > limits.max_output_bytes {
                    truncated_output = true;
                    break;
                }
                doc_text.push('\n');
            }

            let start = doc_text.len();
            let remaining = limits.max_output_bytes.saturating_sub(doc_text.len());
            let text = if p.len() > remaining {
                truncated_output = true;
                docprims_core::truncate_to_utf8_boundary(&p, remaining).to_string()
            } else {
                p
            };
            doc_text.push_str(&text);
            let end = doc_text.len();

            let loc =
                docprims_core::DocprimsLocation::archive("pptx:locator", source_uri, slide_path)
                    .with_hint_u64("block_index", block_index as u64)
                    .with_hint_u64("slide_index", (slide_idx0 + 1) as u64)
                    .with_hint_u64("paragraph_index", p_idx as u64);

            blocks.push(DocprimsBlock {
                id: format!("pptx:paragraph:{}", block_index),
                kind: "pptx:paragraph".to_string(),
                text,
                doc_text_range: DocprimsByteRange {
                    start_byte: start,
                    end_byte: end,
                },
                loc,
                children: vec![],
                role: None,
            });

            block_index += 1;
            if truncated_output {
                break;
            }
        }

        if hit_max_blocks || truncated_output {
            break;
        }
    }

    if hit_max_blocks {
        quality = DocprimsQuality::partial(docprims_core::DOCPRIMS_V0_PARTIAL_MAX_BLOCKS);
        warnings.push(docprims_core::DOCPRIMS_V0_WARN_TRUNCATED_MAX_BLOCKS.to_string());
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
                family: "ooxml".to_string(),
                kind: "pptx".to_string(),
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
    let min_len = parts.len();
    for seg in target.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if parts.len() > min_len {
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

fn extract_slide_paragraphs(xml: &str) -> Result<Vec<String>> {
    let mut reader = Reader::from_str(xml);
    // Don't use trim_text - it drops entity references in quick-xml 0.39

    let mut paras: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                if e.local_name().as_ref() == b"p" {
                    let trimmed = current.trim();
                    if !trimmed.is_empty() {
                        paras.push(trimmed.to_string());
                    }
                    current.clear();
                }
            }
            Ok(Event::End(e)) => {
                if e.local_name().as_ref() == b"p" {
                    let trimmed = current.trim();
                    if !trimmed.is_empty() {
                        paras.push(trimmed.to_string());
                    }
                    current.clear();
                }
            }
            Ok(Event::Text(e)) => {
                let decoded = e
                    .decode()
                    .map_err(|e| DocprimsError::Parse(format!("XML decode error: {}", e)))?;
                current.push_str(&decoded);
            }
            Ok(Event::GeneralRef(e)) => {
                if let Some(resolved) = resolve_entity(&e) {
                    current.push_str(resolved);
                }
            }
            Ok(Event::Eof) => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    paras.push(trimmed.to_string());
                }
                break;
            }
            Err(e) => {
                return Err(DocprimsError::Parse(format!(
                    "Slide parse error at {}: {}",
                    reader.buffer_position(),
                    e
                )));
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(paras)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

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

    #[test]
    fn test_resolve_ooxml_target_does_not_escape_base() {
        let p = resolve_ooxml_target("ppt", "../slides/slide1.xml");
        assert_eq!(p, "ppt/slides/slide1.xml");

        let p = resolve_ooxml_target("ppt", "../../slides/slide1.xml");
        assert_eq!(p, "ppt/slides/slide1.xml");

        let p = resolve_ooxml_target("ppt", "/slides/slide1.xml");
        assert_eq!(p, "ppt/slides/slide1.xml");

        let p = resolve_ooxml_target("ppt", "./slides/slide1.xml");
        assert_eq!(p, "ppt/slides/slide1.xml");

        let p = resolve_ooxml_target("ppt", "slides//slide1.xml");
        assert_eq!(p, "ppt/slides/slide1.xml");

        let p = resolve_ooxml_target("ppt", "slides/../slides/slide1.xml");
        assert_eq!(p, "ppt/slides/slide1.xml");
    }

    #[test]
    fn v0_extracts_paragraphs_with_slide_hints_and_validates_schema() {
        let pres = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:sldIdLst>
    <p:sldId id="256" r:id="rId2"/>
  </p:sldIdLst>
</p:presentation>"#;

        let rels = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Target="slides/slide1.xml"/>
</Relationships>"#;

        let slide = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:sp>
        <p:txBody>
          <a:p><a:r><a:t>Title</a:t></a:r></a:p>
          <a:p><a:r><a:t>Bullet</a:t></a:r></a:p>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

        let zip = crate::test_support::build_zip(vec![
            ("ppt/presentation.xml", pres.as_bytes().to_vec()),
            ("ppt/_rels/presentation.xml.rels", rels.as_bytes().to_vec()),
            ("ppt/slides/slide1.xml", slide.as_bytes().to_vec()),
        ]);

        let out = extract_v0(
            Cursor::new(zip.into_inner()),
            "./x.pptx",
            ExtractLimits {
                max_input_bytes: 1024 * 1024,
                max_output_bytes: 1024 * 1024,
                max_blocks: 100,
            },
        )
        .unwrap();

        assert!(out.document.text.contains("Title"));
        assert!(out.document.text.contains("Bullet"));
        assert_eq!(out.document.blocks[0].kind, "pptx:paragraph");
        assert_eq!(
            out.document.blocks[0]
                .loc
                .hints
                .get("slide_index")
                .and_then(|v| v.as_u64()),
            Some(1)
        );
        crate::test_support::assert_v0_schema_valid(&out);
    }
}
