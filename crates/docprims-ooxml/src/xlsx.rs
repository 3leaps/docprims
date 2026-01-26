//! XLSX (Excel) text extraction.

use crate::common::{open_archive, read_archive_file, resolve_entity};
use docprims_core::{DocprimsError, ExtractedText, Result};
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

    let sheet_paths = enumerate_sheets(&mut archive)?;
    for (idx, sheet_path) in sheet_paths.iter().enumerate() {
        if let Some(sheet_xml) = read_archive_file(&mut archive, sheet_path)? {
            if idx > 0 && !all_text.ends_with('\n') {
                all_text.push('\n');
            }
            let sheet_text = extract_sheet_text(&sheet_xml, &shared_strings)?;
            all_text.push_str(&sheet_text);
        }
    }

    Ok(ExtractedText::complete(all_text.trim().to_string()))
}

fn enumerate_sheets<R: Read + Seek>(archive: &mut zip::ZipArchive<R>) -> Result<Vec<String>> {
    let workbook_xml = read_archive_file(archive, WORKBOOK_PATH)?
        .ok_or_else(|| DocprimsError::Malformed("Missing xl/workbook.xml".to_string()))?;
    let rels_xml = read_archive_file(archive, WORKBOOK_RELS_PATH)?.ok_or_else(|| {
        DocprimsError::Malformed("Missing xl/_rels/workbook.xml.rels".to_string())
    })?;

    let sheet_rids = parse_workbook_sheet_rids(&workbook_xml)?;
    let rels = parse_relationships(&rels_xml)?;

    let mut out = Vec::new();
    for rid in sheet_rids {
        if let Some(target) = rels.get(&rid) {
            out.push(resolve_ooxml_target("xl", target));
        }
    }
    Ok(out)
}

fn parse_workbook_sheet_rids(xml: &str) -> Result<Vec<String>> {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut rids = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(e)) | Ok(Event::Start(e)) => {
                if e.local_name().as_ref() == b"sheet" {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"r:id" || attr.key.as_ref() == b"id" {
                            rids.push(String::from_utf8_lossy(&attr.value).to_string());
                        }
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
                // Don't allow escaping the base dir.
                if !parts.is_empty() {
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

        let rids = parse_workbook_sheet_rids(workbook).unwrap();
        assert_eq!(rids, vec!["rId1".to_string(), "rId2".to_string()]);

        let map = parse_relationships(rels).unwrap();
        assert_eq!(map.get("rId1").unwrap(), "worksheets/sheet1.xml");

        let p1 = resolve_ooxml_target("xl", map.get("rId1").unwrap());
        let p2 = resolve_ooxml_target("xl", map.get("rId2").unwrap());
        assert_eq!(p1, "xl/worksheets/sheet1.xml");
        assert_eq!(p2, "xl/worksheets/sheet2.xml");
    }
}
