//! GPL-free text extraction with provenance.
//!
//! `docprims` extracts text from Markdown, HTML, XML, DOCX, XLSX and PPTX
//! documents into the `extract/v0` structure: the document text, blocks with
//! byte ranges into that text, and a locator for where each block came from.
//! Input is treated as untrusted; [`ExtractLimits`] bounds input size, output
//! size and block count.
//!
//! ```no_run
//! let extract = docprims::extract_file("report.docx", docprims::ExtractLimits::default())?;
//! for block in &extract.document.blocks {
//!     println!("{}: {}", block.kind, block.text);
//! }
//! # Ok::<(), docprims::DocprimsError>(())
//! ```
//!
//! # Features
//!
//! Each format is a feature; `default` enables all of them.
//!
//! | Feature | Formats |
//! |---------|---------|
//! | `text` | `markdown`, `html`, `xml` |
//! | `ooxml` | `docx`, `xlsx`, `pptx` |
//! | `cli` | all formats, plus the `docprims` binary |
//!
//! Extracting a format whose feature is disabled returns
//! [`DocprimsError::UnsupportedFormat`].
//!
//! This crate is the supported entry point. The `docprims-core`,
//! `docprims-text` and `docprims-ooxml` crates it builds on carry no stability
//! promise beyond what is re-exported here.

#![cfg_attr(docsrs, feature(doc_cfg))]

use std::path::Path;

pub use docprims_core::{
    DocprimsBlock, DocprimsByteRange, DocprimsContainer, DocprimsDocument, DocprimsError,
    DocprimsExtract, DocprimsFormat, DocprimsGenerator, DocprimsLocation, DocprimsQuality,
    DocprimsSource, ExtractLimits, Result,
};

/// A document format docprims can extract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Format {
    /// Markdown (`.md`, `.markdown`)
    Markdown,
    /// HTML (`.html`, `.htm`)
    Html,
    /// XML (`.xml`)
    Xml,
    /// Word document (`.docx`)
    Docx,
    /// Excel workbook (`.xlsx`)
    Xlsx,
    /// PowerPoint presentation (`.pptx`)
    Pptx,
}

impl Format {
    /// Every format, whether or not its feature is enabled.
    pub const ALL: [Format; 6] = [
        Format::Markdown,
        Format::Html,
        Format::Xml,
        Format::Docx,
        Format::Xlsx,
        Format::Pptx,
    ];

    /// The format for a file extension, ignoring ASCII case.
    pub fn from_extension(extension: &str) -> Option<Format> {
        match extension.to_ascii_lowercase().as_str() {
            "md" | "markdown" => Some(Format::Markdown),
            "html" | "htm" => Some(Format::Html),
            "xml" => Some(Format::Xml),
            "docx" => Some(Format::Docx),
            "xlsx" => Some(Format::Xlsx),
            "pptx" => Some(Format::Pptx),
            _ => None,
        }
    }

    /// The format for a path or URI, from its extension.
    pub fn from_path(path: impl AsRef<Path>) -> Option<Format> {
        path.as_ref()
            .extension()
            .and_then(|e| e.to_str())
            .and_then(Format::from_extension)
    }

    /// The format's name as used in `extract/v0` output (`source.format.kind`).
    pub fn name(self) -> &'static str {
        match self {
            Format::Markdown => "markdown",
            Format::Html => "html",
            Format::Xml => "xml",
            Format::Docx => "docx",
            Format::Xlsx => "xlsx",
            Format::Pptx => "pptx",
        }
    }

    /// Whether this build of docprims can extract the format.
    pub fn is_enabled(self) -> bool {
        match self {
            Format::Markdown => cfg!(feature = "markdown"),
            Format::Html => cfg!(feature = "html"),
            Format::Xml => cfg!(feature = "xml"),
            Format::Docx => cfg!(feature = "docx"),
            Format::Xlsx => cfg!(feature = "xlsx"),
            Format::Pptx => cfg!(feature = "pptx"),
        }
    }
}

fn format_for(path: &Path) -> Result<Format> {
    Format::from_path(path).ok_or_else(|| {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();
        DocprimsError::UnknownFormat(ext)
    })
}

#[allow(dead_code)] // unused when every format feature is enabled
fn disabled(format: Format) -> DocprimsError {
    DocprimsError::UnsupportedFormat(format!(
        "{} (docprims built without the `{}` feature)",
        format.name(),
        format.name()
    ))
}

/// Extract a file, choosing the format from its extension.
pub fn extract_file(path: impl AsRef<Path>, limits: ExtractLimits) -> Result<DocprimsExtract> {
    let path = path.as_ref();
    extract_file_as(format_for(path)?, path, limits)
}

/// Extract a file as the given format.
#[cfg_attr(
    not(any(
        feature = "markdown",
        feature = "html",
        feature = "xml",
        feature = "docx",
        feature = "xlsx",
        feature = "pptx"
    )),
    allow(unused_variables)
)]
pub fn extract_file_as(
    format: Format,
    path: impl AsRef<Path>,
    limits: ExtractLimits,
) -> Result<DocprimsExtract> {
    let path = path.as_ref();
    match format {
        #[cfg(feature = "markdown")]
        Format::Markdown => docprims_text::extract_markdown_v0(path, limits),
        #[cfg(feature = "html")]
        Format::Html => docprims_text::extract_html_v0(path, limits),
        #[cfg(feature = "xml")]
        Format::Xml => docprims_text::extract_xml_v0(path, limits),
        #[cfg(feature = "docx")]
        Format::Docx => docprims_ooxml::extract_docx_v0(path, limits),
        #[cfg(feature = "xlsx")]
        Format::Xlsx => docprims_ooxml::extract_xlsx_v0(path, limits),
        #[cfg(feature = "pptx")]
        Format::Pptx => docprims_ooxml::extract_pptx_v0(path, limits),
        #[allow(unreachable_patterns)]
        other => Err(disabled(other)),
    }
}

/// Extract in-memory document bytes, choosing the format from the extension
/// of `source_uri`. `source_uri` is recorded in the output as the source.
pub fn extract_bytes(
    source_uri: &str,
    data: &[u8],
    limits: ExtractLimits,
) -> Result<DocprimsExtract> {
    extract_bytes_as(format_for(Path::new(source_uri))?, source_uri, data, limits)
}

/// Extract in-memory document bytes as the given format.
///
/// Markdown, HTML and XML input must be UTF-8.
#[cfg_attr(
    not(any(
        feature = "markdown",
        feature = "html",
        feature = "xml",
        feature = "docx",
        feature = "xlsx",
        feature = "pptx"
    )),
    allow(unused_variables)
)]
pub fn extract_bytes_as(
    format: Format,
    source_uri: &str,
    data: &[u8],
    limits: ExtractLimits,
) -> Result<DocprimsExtract> {
    if data.len() > limits.max_input_bytes {
        return Err(DocprimsError::ResourceLimit(format!(
            "input exceeds max_input_bytes ({} > {})",
            data.len(),
            limits.max_input_bytes
        )));
    }
    match format {
        #[cfg(feature = "markdown")]
        Format::Markdown => {
            docprims_text::markdown::extract_v0_str(utf8(data, format)?, source_uri, limits)
        }
        #[cfg(feature = "html")]
        Format::Html => {
            docprims_text::html::extract_v0_str(utf8(data, format)?, source_uri, limits)
        }
        #[cfg(feature = "xml")]
        Format::Xml => docprims_text::xml::extract_v0_str(utf8(data, format)?, source_uri, limits),
        #[cfg(feature = "docx")]
        Format::Docx => {
            docprims_ooxml::extract_docx_v0_reader(std::io::Cursor::new(data), source_uri, limits)
        }
        #[cfg(feature = "xlsx")]
        Format::Xlsx => {
            docprims_ooxml::extract_xlsx_v0_reader(std::io::Cursor::new(data), source_uri, limits)
        }
        #[cfg(feature = "pptx")]
        Format::Pptx => {
            docprims_ooxml::extract_pptx_v0_reader(std::io::Cursor::new(data), source_uri, limits)
        }
        #[allow(unreachable_patterns)]
        other => Err(disabled(other)),
    }
}

#[cfg(any(feature = "markdown", feature = "html", feature = "xml"))]
fn utf8(data: &[u8], format: Format) -> Result<&str> {
    std::str::from_utf8(data)
        .map_err(|_| DocprimsError::Malformed(format!("non-utf8 {} input", format.name())))
}
