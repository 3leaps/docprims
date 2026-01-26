//! # docprims-ooxml
//!
//! Office Open XML (OOXML) text extraction for docprims.
//!
//! This crate extracts text from:
//! - DOCX (Word documents)
//! - XLSX (Excel spreadsheets)
//! - PPTX (PowerPoint presentations)
//!
//! ## Feature Flags
//!
//! - `docx` - Enable DOCX extraction (default)
//! - `xlsx` - Enable XLSX extraction (default)
//! - `pptx` - Enable PPTX extraction (default)
//!
//! ## Example
//!
//! ```no_run
//! use docprims_ooxml::extract_docx;
//!
//! let text = extract_docx("document.docx").unwrap();
//! println!("{}", text.content);
//! ```

use docprims_core::{DocprimsError, ExtractedText, Result};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

pub mod common;

#[cfg(feature = "docx")]
pub mod docx;

#[cfg(feature = "xlsx")]
pub mod xlsx;

#[cfg(feature = "pptx")]
pub mod pptx;

/// Detect OOXML format from file extension.
pub fn detect_format(path: impl AsRef<Path>) -> Option<OoxmlFormat> {
    let ext = path.as_ref().extension()?.to_str()?.to_lowercase();
    match ext.as_str() {
        "docx" => Some(OoxmlFormat::Docx),
        "xlsx" => Some(OoxmlFormat::Xlsx),
        "pptx" => Some(OoxmlFormat::Pptx),
        _ => None,
    }
}

/// OOXML format types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OoxmlFormat {
    Docx,
    Xlsx,
    Pptx,
}

/// Extract text from an OOXML file (auto-detects format).
pub fn extract(path: impl AsRef<Path>) -> Result<ExtractedText> {
    let path = path.as_ref();
    match detect_format(path) {
        Some(OoxmlFormat::Docx) => extract_docx(path),
        Some(OoxmlFormat::Xlsx) => extract_xlsx(path),
        Some(OoxmlFormat::Pptx) => extract_pptx(path),
        None => Err(DocprimsError::UnknownFormat(path.display().to_string())),
    }
}

/// Extract text from a DOCX file.
#[cfg(feature = "docx")]
pub fn extract_docx(path: impl AsRef<Path>) -> Result<ExtractedText> {
    let file = File::open(path.as_ref())?;
    let reader = BufReader::new(file);
    extract_docx_reader(reader)
}

/// Extract text from a DOCX reader.
#[cfg(feature = "docx")]
pub fn extract_docx_reader<R: Read + std::io::Seek>(reader: R) -> Result<ExtractedText> {
    docx::extract(reader)
}

/// Extract text from an XLSX file.
#[cfg(feature = "xlsx")]
pub fn extract_xlsx(path: impl AsRef<Path>) -> Result<ExtractedText> {
    let file = File::open(path.as_ref())?;
    let reader = BufReader::new(file);
    extract_xlsx_reader(reader)
}

/// Extract text from an XLSX reader.
#[cfg(feature = "xlsx")]
pub fn extract_xlsx_reader<R: Read + std::io::Seek>(reader: R) -> Result<ExtractedText> {
    xlsx::extract(reader)
}

/// Extract text from a PPTX file.
#[cfg(feature = "pptx")]
pub fn extract_pptx(path: impl AsRef<Path>) -> Result<ExtractedText> {
    let file = File::open(path.as_ref())?;
    let reader = BufReader::new(file);
    extract_pptx_reader(reader)
}

/// Extract text from a PPTX reader.
#[cfg(feature = "pptx")]
pub fn extract_pptx_reader<R: Read + std::io::Seek>(reader: R) -> Result<ExtractedText> {
    pptx::extract(reader)
}
