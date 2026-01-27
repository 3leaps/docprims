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

use docprims_core::{DocprimsError, DocprimsExtract, ExtractLimits, ExtractedText, Result};
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

/// Extract structured output (v0 contract) from a DOCX file.
#[cfg(feature = "docx")]
pub fn extract_docx_v0(path: impl AsRef<Path>, limits: ExtractLimits) -> Result<DocprimsExtract> {
    let path = path.as_ref();
    let meta = std::fs::metadata(path)?;
    if meta.len() as usize > limits.max_input_bytes {
        return Err(DocprimsError::ResourceLimit(format!(
            "input exceeds max_input_bytes ({} > {})",
            meta.len(),
            limits.max_input_bytes
        )));
    }

    let file = File::open(path)?;
    let reader = BufReader::new(file);
    extract_docx_v0_reader(reader, &path.display().to_string(), limits)
}

/// Extract text from a DOCX reader.
#[cfg(feature = "docx")]
pub fn extract_docx_reader<R: Read + std::io::Seek>(reader: R) -> Result<ExtractedText> {
    docx::extract(reader)
}

/// Extract structured output (v0 contract) from a DOCX reader.
#[cfg(feature = "docx")]
pub fn extract_docx_v0_reader<R: Read + std::io::Seek>(
    reader: R,
    source_uri: &str,
    limits: ExtractLimits,
) -> Result<DocprimsExtract> {
    docx::extract_v0(reader, source_uri, limits)
}

/// Extract text from an XLSX file.
#[cfg(feature = "xlsx")]
pub fn extract_xlsx(path: impl AsRef<Path>) -> Result<ExtractedText> {
    let file = File::open(path.as_ref())?;
    let reader = BufReader::new(file);
    extract_xlsx_reader(reader)
}

/// Extract structured output (v0 contract) from an XLSX file.
#[cfg(feature = "xlsx")]
pub fn extract_xlsx_v0(path: impl AsRef<Path>, limits: ExtractLimits) -> Result<DocprimsExtract> {
    let path = path.as_ref();
    let meta = std::fs::metadata(path)?;
    if meta.len() as usize > limits.max_input_bytes {
        return Err(DocprimsError::ResourceLimit(format!(
            "input exceeds max_input_bytes ({} > {})",
            meta.len(),
            limits.max_input_bytes
        )));
    }

    let file = File::open(path)?;
    let reader = BufReader::new(file);
    extract_xlsx_v0_reader(reader, &path.display().to_string(), limits)
}

/// Extract text from an XLSX reader.
#[cfg(feature = "xlsx")]
pub fn extract_xlsx_reader<R: Read + std::io::Seek>(reader: R) -> Result<ExtractedText> {
    xlsx::extract(reader)
}

/// Extract structured output (v0 contract) from an XLSX reader.
#[cfg(feature = "xlsx")]
pub fn extract_xlsx_v0_reader<R: Read + std::io::Seek>(
    reader: R,
    source_uri: &str,
    limits: ExtractLimits,
) -> Result<DocprimsExtract> {
    xlsx::extract_v0(reader, source_uri, limits)
}

/// Extract text from a PPTX file.
#[cfg(feature = "pptx")]
pub fn extract_pptx(path: impl AsRef<Path>) -> Result<ExtractedText> {
    let file = File::open(path.as_ref())?;
    let reader = BufReader::new(file);
    extract_pptx_reader(reader)
}

/// Extract structured output (v0 contract) from a PPTX file.
#[cfg(feature = "pptx")]
pub fn extract_pptx_v0(path: impl AsRef<Path>, limits: ExtractLimits) -> Result<DocprimsExtract> {
    let path = path.as_ref();
    let meta = std::fs::metadata(path)?;
    if meta.len() as usize > limits.max_input_bytes {
        return Err(DocprimsError::ResourceLimit(format!(
            "input exceeds max_input_bytes ({} > {})",
            meta.len(),
            limits.max_input_bytes
        )));
    }

    let file = File::open(path)?;
    let reader = BufReader::new(file);
    extract_pptx_v0_reader(reader, &path.display().to_string(), limits)
}

/// Extract text from a PPTX reader.
#[cfg(feature = "pptx")]
pub fn extract_pptx_reader<R: Read + std::io::Seek>(reader: R) -> Result<ExtractedText> {
    pptx::extract(reader)
}

/// Extract structured output (v0 contract) from a PPTX reader.
#[cfg(feature = "pptx")]
pub fn extract_pptx_v0_reader<R: Read + std::io::Seek>(
    reader: R,
    source_uri: &str,
    limits: ExtractLimits,
) -> Result<DocprimsExtract> {
    pptx::extract_v0(reader, source_uri, limits)
}

#[cfg(test)]
pub(crate) mod test_support {
    use docprims_core::DocprimsExtract;
    use jsonschema::Resource;
    use std::io::{Cursor, Write};
    use zip::write::SimpleFileOptions;

    pub(crate) fn assert_v0_schema_valid(extract: &DocprimsExtract) {
        let root_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../schemas/v0/extract/docprims-extract.schema.json"
        ))
        .unwrap();

        let block_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../schemas/v0/extract/docprims-block.schema.json"
        ))
        .unwrap();

        let loc_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../schemas/v0/extract/docprims-location.schema.json"
        ))
        .unwrap();

        let validator = jsonschema::draft202012::options()
            .with_resources([
                (
                    "https://schemas.3leaps.dev/docprims/extract/v0/docprims-block.schema.json",
                    Resource::from_contents(block_schema).unwrap(),
                ),
                (
                    "https://schemas.3leaps.dev/docprims/extract/v0/docprims-location.schema.json",
                    Resource::from_contents(loc_schema).unwrap(),
                ),
            ]
            .into_iter())
            .build(&root_schema)
            .unwrap();

        let value = serde_json::to_value(extract).unwrap();
        let errors: Vec<_> = validator.iter_errors(&value).collect();
        assert!(errors.is_empty(), "schema errors: {errors:?}");
    }

    pub(crate) fn build_zip(files: Vec<(&str, Vec<u8>)>) -> Cursor<Vec<u8>> {
        let mut buf = Cursor::new(Vec::new());
        {
            let mut w = zip::ZipWriter::new(&mut buf);
            let opts = SimpleFileOptions::default();
            for (name, bytes) in files {
                w.start_file(name, opts).unwrap();
                w.write_all(&bytes).unwrap();
            }
            w.finish().unwrap();
        }
        buf.set_position(0);
        buf
    }

    pub(crate) fn utf16le_with_bom(s: &str) -> Vec<u8> {
        let mut out = Vec::with_capacity(2 + s.len() * 2);
        out.extend_from_slice(&[0xFF, 0xFE]);
        for u in s.encode_utf16() {
            out.extend_from_slice(&u.to_le_bytes());
        }
        out
    }
}
