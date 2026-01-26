//! XLSX (Excel) text extraction.

use crate::common::{open_archive, read_archive_file, resolve_entity};
use docprims_core::{DocprimsError, ExtractedText, Result};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::io::{Read, Seek};

/// Shared strings path in XLSX archive.
const SHARED_STRINGS_PATH: &str = "xl/sharedStrings.xml";

/// First sheet path (default).
const SHEET1_PATH: &str = "xl/worksheets/sheet1.xml";

/// Extract text from an XLSX reader.
pub fn extract<R: Read + Seek>(reader: R) -> Result<ExtractedText> {
    let mut archive = open_archive(reader)?;

    // Load shared strings table
    let shared_strings = match read_archive_file(&mut archive, SHARED_STRINGS_PATH)? {
        Some(xml) => parse_shared_strings(&xml)?,
        None => Vec::new(),
    };

    // Extract text from sheets
    // TODO: Enumerate all sheets from workbook.xml
    let mut all_text = String::new();

    if let Some(sheet_xml) = read_archive_file(&mut archive, SHEET1_PATH)? {
        let sheet_text = extract_sheet_text(&sheet_xml, &shared_strings)?;
        all_text.push_str(&sheet_text);
    }

    Ok(ExtractedText::complete(all_text.trim().to_string()))
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
}
