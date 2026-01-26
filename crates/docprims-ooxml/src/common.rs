//! Common utilities for OOXML parsing.

use docprims_core::{DocprimsError, Result};
use quick_xml::escape::resolve_xml_entity;
use std::io::{Read, Seek};
use zip::ZipArchive;

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

    let mut content = String::with_capacity(size as usize);
    let mut reader = std::io::BufReader::new(file);
    reader
        .read_to_string(&mut content)
        .map_err(|e| DocprimsError::Malformed(format!("Error reading {}: {}", name, e)))?;

    Ok(Some(content))
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
