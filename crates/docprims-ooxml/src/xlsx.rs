//! XLSX (Excel) text extraction.

use crate::common::{open_archive, read_archive_file, resolve_entity};
use docprims_core::{
    DocprimsBlock, DocprimsByteRange, DocprimsDocument, DocprimsError, DocprimsExtract,
    DocprimsGenerator, DocprimsQuality, DocprimsSource, ExtractLimits, ExtractedText, Result,
};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::collections::HashMap;
use std::io::{Read, Seek};

/// Shared strings path in XLSX archive.
const SHARED_STRINGS_PATH: &str = "xl/sharedStrings.xml";

/// Workbook path in XLSX archive.
const WORKBOOK_PATH: &str = "xl/workbook.xml";

/// Workbook relationships path.
const WORKBOOK_RELS_PATH: &str = "xl/_rels/workbook.xml.rels";

/// Extract text from an XLSX reader.
pub fn extract<R: Read + Seek>(reader: R) -> Result<ExtractedText> {
    let mut archive = open_archive(reader)?;

    // Load shared strings table
    let shared_strings = match read_archive_file(&mut archive, SHARED_STRINGS_PATH)? {
        Some(xml) => parse_shared_strings(&xml)?,
        None => Vec::new(),
    };

    let mut all_text = String::new();

    let sheets = enumerate_sheets(&mut archive)?;
    for (idx, sheet) in sheets.iter().enumerate() {
        if let Some(sheet_xml) = read_archive_file(&mut archive, &sheet.path)? {
            if idx > 0 && !all_text.ends_with('\n') {
                all_text.push('\n');
            }
            let sheet_text = extract_sheet_text(&sheet_xml, &shared_strings)?;
            all_text.push_str(&sheet_text);
        }
    }

    Ok(ExtractedText::complete(all_text.trim().to_string()))
}

#[derive(Debug, Clone)]
struct SheetInfo {
    index: usize,
    name: String,
    path: String,
}

#[derive(Debug, Clone)]
struct SheetRow {
    row_index: Option<u64>,
    text: String,
}

/// Extract structured output (v0 contract) from an XLSX reader.
pub fn extract_v0<R: Read + Seek>(
    reader: R,
    source_uri: &str,
    limits: ExtractLimits,
) -> Result<DocprimsExtract> {
    let mut archive = open_archive(reader)?;

    let shared_strings = match read_archive_file(&mut archive, SHARED_STRINGS_PATH)? {
        Some(xml) => parse_shared_strings(&xml)?,
        None => Vec::new(),
    };

    let sheets = enumerate_sheets(&mut archive)?;

    let mut warnings = Vec::new();
    let mut quality = DocprimsQuality::complete();
    let mut blocks = Vec::new();
    let mut doc_text = String::new();

    let mut hit_max_blocks = false;
    let mut truncated_output = false;
    let mut block_index = 0usize;

    for sheet in sheets {
        if block_index >= limits.max_blocks {
            hit_max_blocks = true;
            break;
        }

        let sheet_index = sheet.index;
        let sheet_name = sheet.name.clone();
        let sheet_path = sheet.path.clone();

        let Some(sheet_xml) = read_archive_file(&mut archive, &sheet_path)? else {
            continue;
        };

        let rows = extract_sheet_rows(&sheet_xml, &shared_strings)?;
        for row in rows {
            if row.text.is_empty() {
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
            let text = if row.text.len() > remaining {
                truncated_output = true;
                docprims_core::truncate_to_utf8_boundary(&row.text, remaining).to_string()
            } else {
                row.text
            };
            doc_text.push_str(&text);
            let end = doc_text.len();

            let mut loc =
                docprims_core::DocprimsLocation::archive("xlsx:row", source_uri, &sheet_path)
                    .with_hint_u64("block_index", block_index as u64)
                    .with_hint_u64("sheet_index", sheet_index as u64)
                    .with_hint_str("sheet_name", sheet_name.clone());
            if let Some(r) = row.row_index {
                loc = loc.with_hint_u64("row_index", r);
            }

            blocks.push(DocprimsBlock {
                id: format!("xlsx:row:{}", block_index),
                kind: "xlsx:row".to_string(),
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
                kind: "xlsx".to_string(),
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

fn enumerate_sheets<R: Read + Seek>(archive: &mut zip::ZipArchive<R>) -> Result<Vec<SheetInfo>> {
    let workbook_xml = read_archive_file(archive, WORKBOOK_PATH)?
        .ok_or_else(|| DocprimsError::Malformed("Missing xl/workbook.xml".to_string()))?;
    let rels_xml = read_archive_file(archive, WORKBOOK_RELS_PATH)?.ok_or_else(|| {
        DocprimsError::Malformed("Missing xl/_rels/workbook.xml.rels".to_string())
    })?;

    let sheet_entries = parse_workbook_sheets(&workbook_xml)?;
    let rels = parse_relationships(&rels_xml)?;

    let mut out = Vec::new();
    for (index, name, rid) in sheet_entries {
        if let Some(target) = rels.get(&rid) {
            out.push(SheetInfo {
                index,
                name,
                path: resolve_ooxml_target("xl", target),
            });
        }
    }
    Ok(out)
}

fn parse_workbook_sheets(xml: &str) -> Result<Vec<(usize, String, String)>> {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut out = Vec::new();
    let mut index = 0usize;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(e)) | Ok(Event::Start(e)) => {
                if e.local_name().as_ref() == b"sheet" {
                    let mut name: Option<String> = None;
                    let mut rid: Option<String> = None;
                    for attr in e.attributes().flatten() {
                        match attr.key.as_ref() {
                            b"name" => {
                                name = Some(String::from_utf8_lossy(&attr.value).to_string())
                            }
                            b"r:id" => rid = Some(String::from_utf8_lossy(&attr.value).to_string()),
                            _ => {}
                        }
                    }

                    if let Some(rid) = rid {
                        out.push((
                            index,
                            name.unwrap_or_else(|| format!("Sheet{}", index + 1)),
                            rid,
                        ));
                        index += 1;
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(DocprimsError::Parse(format!(
                    "workbook.xml parse error at {}: {}",
                    reader.buffer_position(),
                    e
                )));
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(out)
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
                // Don't allow escaping the base dir.
                if parts.len() > min_len {
                    parts.pop();
                }
            }
            _ => parts.push(seg),
        }
    }

    parts.join("/")
}

/// Parse the shared strings table.
fn parse_shared_strings(xml: &str) -> Result<Vec<String>> {
    let mut reader = Reader::from_str(xml);
    // Don't use trim_text - it drops entity references in quick-xml 0.39

    let mut strings = Vec::new();
    let mut buf = Vec::new();
    let mut current_string = String::new();
    let mut in_si = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local_name = e.local_name();
                if local_name.as_ref() == b"si" {
                    in_si = true;
                    current_string.clear();
                }
            }
            Ok(Event::End(e)) => {
                let local_name = e.local_name();
                if local_name.as_ref() == b"si" {
                    strings.push(current_string.trim().to_string());
                    in_si = false;
                }
            }
            Ok(Event::Text(e)) => {
                if in_si {
                    let decoded = e
                        .decode()
                        .map_err(|e| DocprimsError::Parse(format!("XML decode error: {}", e)))?;
                    current_string.push_str(&decoded);
                }
            }
            Ok(Event::GeneralRef(e)) => {
                if in_si {
                    if let Some(resolved) = resolve_entity(&e) {
                        current_string.push_str(resolved);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(DocprimsError::Parse(format!(
                    "Shared strings parse error: {}",
                    e
                )));
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(strings)
}

fn extract_sheet_rows(xml: &str, shared_strings: &[String]) -> Result<Vec<SheetRow>> {
    let mut reader = Reader::from_str(xml);
    // Don't use trim_text - it drops entity references in quick-xml 0.39

    let mut rows = Vec::new();
    let mut buf = Vec::new();

    let mut in_row = false;
    let mut row_index: Option<u64> = None;
    let mut row_text = String::new();

    let mut cell_type: Option<String> = None;
    let mut cell_value = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.local_name().as_ref() {
                b"row" => {
                    in_row = true;
                    row_text.clear();
                    row_index = None;
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"r" {
                            row_index = String::from_utf8_lossy(&attr.value).parse::<u64>().ok();
                        }
                    }
                }
                b"c" => {
                    cell_type = None;
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"t" {
                            cell_type = Some(String::from_utf8_lossy(&attr.value).to_string());
                        }
                    }
                    cell_value.clear();
                }
                _ => {}
            },
            Ok(Event::End(e)) => match e.local_name().as_ref() {
                b"c" => {
                    if in_row {
                        let value =
                            resolve_cell_value(cell_value.trim(), &cell_type, shared_strings);
                        if !value.is_empty() {
                            if !row_text.is_empty() {
                                row_text.push('\t');
                            }
                            row_text.push_str(&value);
                        }
                    }
                }
                b"row" => {
                    in_row = false;
                    let t = row_text.trim_end().to_string();
                    if !t.is_empty() {
                        rows.push(SheetRow { row_index, text: t });
                    }
                    row_text.clear();
                    row_index = None;
                }
                _ => {}
            },
            Ok(Event::Text(e)) => {
                let decoded = e
                    .decode()
                    .map_err(|e| DocprimsError::Parse(format!("XML decode error: {}", e)))?;
                cell_value.push_str(&decoded);
            }
            Ok(Event::GeneralRef(e)) => {
                if let Some(resolved) = resolve_entity(&e) {
                    cell_value.push_str(resolved);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(DocprimsError::Parse(format!(
                    "Sheet parse error at {}: {}",
                    reader.buffer_position(),
                    e
                )));
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(rows)
}

/// Extract text from a sheet.
fn extract_sheet_text(xml: &str, shared_strings: &[String]) -> Result<String> {
    let mut reader = Reader::from_str(xml);
    // Don't use trim_text - it drops entity references in quick-xml 0.39

    let mut text = String::new();
    let mut buf = Vec::new();
    let mut cell_type: Option<String> = None;
    let mut cell_value = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local_name = e.local_name();
                match local_name.as_ref() {
                    b"row" => {
                        if !text.is_empty() && !text.ends_with('\n') {
                            text.push('\n');
                        }
                    }
                    b"c" => {
                        // Cell - check type attribute
                        cell_type = None;
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"t" {
                                cell_type = Some(String::from_utf8_lossy(&attr.value).to_string());
                            }
                        }
                        cell_value.clear();
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let local_name = e.local_name();
                if local_name.as_ref() == b"c" {
                    // Output cell value
                    let value = resolve_cell_value(cell_value.trim(), &cell_type, shared_strings);
                    if !value.is_empty() {
                        if !text.is_empty() && !text.ends_with('\n') && !text.ends_with('\t') {
                            text.push('\t');
                        }
                        text.push_str(&value);
                    }
                }
            }
            Ok(Event::Text(e)) => {
                let decoded = e
                    .decode()
                    .map_err(|e| DocprimsError::Parse(format!("XML decode error: {}", e)))?;
                cell_value.push_str(&decoded);
            }
            Ok(Event::GeneralRef(e)) => {
                if let Some(resolved) = resolve_entity(&e) {
                    cell_value.push_str(resolved);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(DocprimsError::Parse(format!("Sheet parse error: {}", e)));
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(text)
}

/// Resolve cell value based on type.
fn resolve_cell_value(
    value: &str,
    cell_type: &Option<String>,
    shared_strings: &[String],
) -> String {
    match cell_type.as_deref() {
        Some("s") => {
            // Shared string index
            if let Ok(idx) = value.parse::<usize>() {
                shared_strings.get(idx).cloned().unwrap_or_default()
            } else {
                value.to_string()
            }
        }
        Some("b") => {
            // Boolean
            if value == "1" {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        _ => {
            // Number or inline string
            value.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_parse_shared_strings() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <si><t>Hello</t></si>
  <si><t>World</t></si>
</sst>"#;

        let strings = parse_shared_strings(xml).unwrap();
        assert_eq!(strings.len(), 2);
        assert_eq!(strings[0], "Hello");
        assert_eq!(strings[1], "World");
    }

    #[test]
    fn test_resolve_shared_string() {
        let shared = vec!["Apple".to_string(), "Banana".to_string()];
        let value = resolve_cell_value("0", &Some("s".to_string()), &shared);
        assert_eq!(value, "Apple");
    }

    #[test]
    fn test_resolve_number() {
        let value = resolve_cell_value("42.5", &None, &[]);
        assert_eq!(value, "42.5");
    }

    #[test]
    fn test_enumerate_sheets_from_rels_and_workbook() {
        let workbook = r#"<?xml version="1.0" encoding="UTF-8"?>
<workbook xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets>
    <sheet name="Sheet1" sheetId="1" r:id="rId1"/>
    <sheet name="Sheet2" sheetId="2" r:id="rId2"/>
  </sheets>
</workbook>"#;

        let rels = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Target="worksheets/sheet1.xml"/>
  <Relationship Id="rId2" Target="worksheets/sheet2.xml"/>
</Relationships>"#;

        let sheets = parse_workbook_sheets(workbook).unwrap();
        assert_eq!(
            sheets,
            vec![
                (0, "Sheet1".to_string(), "rId1".to_string()),
                (1, "Sheet2".to_string(), "rId2".to_string())
            ]
        );

        let map = parse_relationships(rels).unwrap();
        assert_eq!(map.get("rId1").unwrap(), "worksheets/sheet1.xml");

        let p1 = resolve_ooxml_target("xl", map.get("rId1").unwrap());
        let p2 = resolve_ooxml_target("xl", map.get("rId2").unwrap());
        assert_eq!(p1, "xl/worksheets/sheet1.xml");
        assert_eq!(p2, "xl/worksheets/sheet2.xml");
    }

    #[test]
    fn test_resolve_ooxml_target_does_not_escape_base() {
        let p = resolve_ooxml_target("xl", "../worksheets/sheet1.xml");
        assert_eq!(p, "xl/worksheets/sheet1.xml");

        let p = resolve_ooxml_target("xl", "../../worksheets/sheet1.xml");
        assert_eq!(p, "xl/worksheets/sheet1.xml");

        let p = resolve_ooxml_target("xl", "/worksheets/sheet1.xml");
        assert_eq!(p, "xl/worksheets/sheet1.xml");

        let p = resolve_ooxml_target("xl", "./worksheets/sheet1.xml");
        assert_eq!(p, "xl/worksheets/sheet1.xml");

        let p = resolve_ooxml_target("xl", "worksheets//sheet1.xml");
        assert_eq!(p, "xl/worksheets/sheet1.xml");

        let p = resolve_ooxml_target("xl", "worksheets/../worksheets/sheet1.xml");
        assert_eq!(p, "xl/worksheets/sheet1.xml");
    }

    #[test]
    fn test_extract_sheet_rows() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
    <row r="1">
      <c t="s"><v>0</v></c>
      <c><v>42</v></c>
    </row>
    <row r="2">
      <c><v></v></c>
      <c t="s"><v>1</v></c>
    </row>
  </sheetData>
</worksheet>"#;
        let shared = vec!["Apple".to_string(), "Banana".to_string()];
        let rows = extract_sheet_rows(xml, &shared).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].row_index, Some(1));
        assert_eq!(rows[0].text, "Apple\t42");
        assert_eq!(rows[1].row_index, Some(2));
        assert_eq!(rows[1].text, "Banana");
    }

    #[test]
    fn v0_extracts_rows_with_sheet_metadata_and_validates_schema() {
        let workbook = r#"<?xml version="1.0" encoding="UTF-8"?>
<workbook xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets>
    <sheet name="Sheet1" sheetId="1" r:id="rId1"/>
  </sheets>
</workbook>"#;

        let rels = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Target="worksheets/sheet1.xml"/>
</Relationships>"#;

        let shared = r#"<?xml version="1.0" encoding="UTF-8"?>
<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <si><t>Apple</t></si>
</sst>"#;

        let sheet1 = r#"<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
    <row r="1">
      <c t="s"><v>0</v></c>
      <c><v>42</v></c>
    </row>
  </sheetData>
</worksheet>"#;

        let zip = crate::test_support::build_zip(vec![
            ("xl/workbook.xml", workbook.as_bytes().to_vec()),
            ("xl/_rels/workbook.xml.rels", rels.as_bytes().to_vec()),
            ("xl/sharedStrings.xml", shared.as_bytes().to_vec()),
            ("xl/worksheets/sheet1.xml", sheet1.as_bytes().to_vec()),
        ]);

        let out = extract_v0(
            Cursor::new(zip.into_inner()),
            "./x.xlsx",
            ExtractLimits {
                max_input_bytes: 1024 * 1024,
                max_output_bytes: 1024 * 1024,
                max_blocks: 100,
            },
        )
        .unwrap();

        assert!(out.document.text.contains("Apple\t42"));
        assert_eq!(out.document.blocks[0].kind, "xlsx:row");
        assert_eq!(
            out.document.blocks[0].loc.container.kind,
            docprims_core::DocprimsContainerKind::Archive
        );
        assert_eq!(
            out.document.blocks[0]
                .loc
                .hints
                .get("sheet_name")
                .and_then(|v| v.as_str()),
            Some("Sheet1")
        );
        crate::test_support::assert_v0_schema_valid(&out);
    }
}
