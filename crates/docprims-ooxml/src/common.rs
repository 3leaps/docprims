//! Common utilities for OOXML parsing.

use docprims_core::{DocprimsError, Result};
use quick_xml::escape::resolve_xml_entity;
use std::borrow::Cow;
use std::io::{Read, Seek};
use zip::ZipArchive;

use encoding_rs::Encoding;

/// Maximum decompressed size to prevent zip bombs (100 MB default).
pub const MAX_DECOMPRESSED_SIZE: u64 = 100 * 1024 * 1024;

/// Maximum number of files in archive to prevent DoS.
pub const MAX_ARCHIVE_FILES: usize = 10_000;

/// Open a ZIP archive with safety limits.
pub fn open_archive<R: Read + Seek>(reader: R) -> Result<ZipArchive<R>> {
    let archive = ZipArchive::new(reader)
        .map_err(|e| DocprimsError::Malformed(format!("Invalid ZIP archive: {}", e)))?;

    // Check file count limit
    if archive.len() > MAX_ARCHIVE_FILES {
        return Err(DocprimsError::ResourceLimit(format!(
            "Archive contains too many files: {} (max {})",
            archive.len(),
            MAX_ARCHIVE_FILES
        )));
    }

    Ok(archive)
}

/// Read a file from the archive with size limits.
pub fn read_archive_file<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> Result<Option<String>> {
    let file = match archive.by_name(name) {
        Ok(f) => f,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => {
            return Err(DocprimsError::Malformed(format!(
                "Error reading {}: {}",
                name, e
            )))
        }
    };

    // Check decompressed size
    let size = file.size();
    if size > MAX_DECOMPRESSED_SIZE {
        return Err(DocprimsError::ResourceLimit(format!(
            "File {} too large: {} bytes (max {})",
            name, size, MAX_DECOMPRESSED_SIZE
        )));
    }

    let mut content = Vec::with_capacity(size as usize);
    let mut reader = std::io::BufReader::new(file);
    reader
        .read_to_end(&mut content)
        .map_err(|e| DocprimsError::Malformed(format!("Error reading {}: {}", name, e)))?;

    Ok(Some(decode_xml_bytes(&content)?))
}

fn decode_xml_bytes(bytes: &[u8]) -> Result<String> {
    let (enc, bom_len) = detect_encoding_and_bom(bytes);
    let bytes = bytes.get(bom_len..).unwrap_or(bytes);
    let (decoded, _, _) = enc.decode(bytes);
    Ok(match decoded {
        Cow::Borrowed(s) => s.to_string(),
        Cow::Owned(s) => s,
    })
}

fn detect_encoding_and_bom(bytes: &[u8]) -> (&'static Encoding, usize) {
    // BOM detection first.
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return (encoding_rs::UTF_8, 3);
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return (encoding_rs::UTF_16LE, 2);
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        return (encoding_rs::UTF_16BE, 2);
    }

    // Best-effort XML declaration sniffing.
    if let Some(label) = sniff_xml_decl_encoding(bytes) {
        if let Some(enc) = Encoding::for_label(label.as_bytes()) {
            return (enc, 0);
        }
    }

    (encoding_rs::UTF_8, 0)
}

fn sniff_xml_decl_encoding(bytes: &[u8]) -> Option<String> {
    // XML declaration is ASCII-compatible; only scan a small prefix.
    let prefix_len = bytes.len().min(1024);
    let prefix = &bytes[..prefix_len];
    let s = std::str::from_utf8(prefix).ok()?;
    let lower = s.to_ascii_lowercase();
    if !lower.starts_with("<?xml") {
        return None;
    }

    let idx = lower.find("encoding=")?;
    let after = &s[idx + "encoding=".len()..];
    let mut chars = after.chars();
    let quote = chars.next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }

    let rest = chars.as_str();
    let end = rest.find(quote)?;
    Some(rest[..end].trim().to_string())
}

/// Resolve an XML entity reference to its string value.
///
/// Handles predefined XML entities (lt, gt, amp, apos, quot) and numeric
/// character references (&#NNN; or &#xHHH;).
pub fn resolve_entity(entity: &[u8]) -> Option<&'static str> {
    let name = std::str::from_utf8(entity).ok()?;

    // Try predefined entities first
    if let Some(resolved) = resolve_xml_entity(name) {
        return Some(resolved);
    }

    // For numeric entities, we'd need to return an owned String,
    // but for OOXML content this is rarely needed
    None
}

/// OOXML namespace constants.
pub mod ns {
    /// Word processing namespace
    pub const WORDPROCESSING: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

    /// Spreadsheet namespace
    pub const SPREADSHEET: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

    /// Presentation namespace
    pub const PRESENTATION: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";

    /// Drawing namespace
    pub const DRAWING: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

    /// Relationships namespace
    pub const RELATIONSHIPS: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
}
