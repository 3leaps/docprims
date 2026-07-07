//! DOCX (Word) text extraction.

use crate::common::{open_archive, read_archive_file, resolve_entity};
use docprims_core::{
    DocprimsBlock, DocprimsByteRange, DocprimsDocument, DocprimsError, DocprimsExtract,
    DocprimsGenerator, DocprimsQuality, DocprimsSource, ExtractLimits, ExtractedText, Result,
};
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

/// Extract structured output (v0 contract) from a DOCX reader.
pub fn extract_v0<R: Read + Seek>(
    reader: R,
    source_uri: &str,
    limits: ExtractLimits,
) -> Result<DocprimsExtract> {
    let mut archive = open_archive(reader)?;

    let document_xml = read_archive_file(&mut archive, DOCUMENT_PATH)?
        .ok_or_else(|| DocprimsError::Malformed("Missing word/document.xml".to_string()))?;

    let paragraphs = extract_paragraphs(&document_xml)?;

    let mut warnings = Vec::new();
    let mut quality = DocprimsQuality::complete();
    let mut blocks = Vec::new();
    let mut doc_text = String::new();

    let mut hit_max_blocks = false;
    let mut truncated_output = false;
    let mut block_index = 0usize;

    for (p_idx, p) in paragraphs.into_iter().enumerate() {
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
            docprims_core::DocprimsLocation::archive("docx:locator", source_uri, DOCUMENT_PATH)
                .with_hint_u64("block_index", block_index as u64)
                .with_hint_u64("paragraph_index", p_idx as u64);

        blocks.push(DocprimsBlock {
            id: format!("docx:paragraph:{}", block_index),
            kind: "docx:paragraph".to_string(),
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
                kind: "docx".to_string(),
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

fn extract_paragraphs(xml: &str) -> Result<Vec<String>> {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut paras = Vec::new();

    let mut in_p = false;
    let mut cur = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.local_name().as_ref() {
                b"p" => {
                    in_p = true;
                    cur.clear();
                }
                b"br" if in_p => {
                    cur.push('\n');
                }
                b"tab" if in_p => {
                    cur.push('\t');
                }
                _ => {}
            },
            Ok(Event::End(e)) => {
                if e.local_name().as_ref() == b"p" {
                    in_p = false;
                    let t = cur.trim().to_string();
                    if !t.is_empty() {
                        paras.push(t);
                    }
                    cur.clear();
                }
            }
            Ok(Event::Text(e)) => {
                if in_p {
                    let decoded = e
                        .decode()
                        .map_err(|e| DocprimsError::Parse(format!("XML decode error: {}", e)))?;
                    cur.push_str(&decoded);
                }
            }
            Ok(Event::GeneralRef(e)) => {
                if in_p {
                    if let Some(resolved) = resolve_entity(&e) {
                        cur.push_str(resolved);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(DocprimsError::Parse(format!(
                    "DOCX parse error at {}: {}",
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

    #[test]
    fn v0_extracts_paragraph_blocks_and_validates_schema() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Hello</w:t></w:r></w:p>
    <w:p><w:r><w:t>World</w:t></w:r></w:p>
  </w:body>
</w:document>"#;

        let zip = crate::test_support::build_zip(vec![(
            "word/document.xml",
            crate::test_support::utf16le_with_bom(xml),
        )]);
        let out = extract_v0(
            Cursor::new(zip.into_inner()),
            "./x.docx",
            ExtractLimits {
                max_input_bytes: 1024 * 1024,
                max_output_bytes: 1024 * 1024,
                max_blocks: 100,
            },
        )
        .unwrap();

        assert!(out.document.text.contains("Hello"));
        assert!(out.document.text.contains("World"));
        assert!(!out.document.blocks.is_empty());
        assert_eq!(
            out.document.blocks[0].loc.container.kind,
            docprims_core::DocprimsContainerKind::Archive
        );
        assert_eq!(
            out.document.blocks[0].loc.container.part.as_deref(),
            Some("word/document.xml")
        );
        crate::test_support::assert_v0_schema_valid(&out);
    }
}
