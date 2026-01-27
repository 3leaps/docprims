//! # docprims-core
//!
//! Core types, traits, and errors for the docprims document extraction library.
//!
//! This crate provides the shared foundation used by format-specific extractors
//! (docprims-text, docprims-ooxml, etc.).

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::Path;
use thiserror::Error;
use time::OffsetDateTime;

pub const DOCPRIMS_V0_EXTRACT_SCHEMA_ID: &str =
    "https://schemas.3leaps.dev/docprims/extract/v0/docprims-extract.schema.json";
pub const DOCPRIMS_V0_SCHEMA_VERSION: &str = "1.0.0";

// Warning and partial-reason strings are part of the v0 JSON contract.
// Keep them stable; golden tests and downstream tooling may depend on them.
pub const DOCPRIMS_V0_WARN_TRUNCATED_MAX_BLOCKS: &str = "docprims:v0:truncated:max_blocks";
pub const DOCPRIMS_V0_WARN_TRUNCATED_MAX_OUTPUT_BYTES: &str =
    "docprims:v0:truncated:max_output_bytes";
pub const DOCPRIMS_V0_PARTIAL_MAX_BLOCKS: &str = "docprims:v0:partial:max_blocks";
pub const DOCPRIMS_V0_PARTIAL_MAX_OUTPUT_BYTES: &str = "docprims:v0:partial:max_output_bytes";

/// Truncate a string to a UTF-8 boundary at or below `max_bytes`.
///
/// This is used to enforce byte-based output limits while preserving valid UTF-8.
pub fn truncate_to_utf8_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }

    let mut last = 0;
    for (i, _) in s.char_indices() {
        if i > max_bytes {
            break;
        }
        last = i;
    }
    &s[..last]
}

/// Errors that can occur during document extraction.
#[derive(Error, Debug)]
pub enum DocprimsError {
    /// I/O error reading the document
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Document format is not supported
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    /// Document format could not be detected
    #[error("Unknown format for file: {0}")]
    UnknownFormat(String),

    /// Document is malformed or corrupted
    #[error("Malformed document: {0}")]
    Malformed(String),

    /// Document is encrypted or password-protected
    #[error("Document is encrypted")]
    Encrypted,

    /// Resource limit exceeded (memory, time, etc.)
    #[error("Resource limit exceeded: {0}")]
    ResourceLimit(String),

    /// Parser-specific error
    #[error("Parse error: {0}")]
    Parse(String),
}

/// Result type for docprims operations.
pub type Result<T> = std::result::Result<T, DocprimsError>;

/// Extracted text from a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedText {
    /// The full extracted text content
    pub content: String,

    /// Document metadata (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<DocumentMetadata>,

    /// Quality indicator for the extraction
    pub quality: ExtractionQuality,
}

impl ExtractedText {
    /// Create a new ExtractedText with complete extraction.
    pub fn complete(content: String) -> Self {
        Self {
            content,
            metadata: None,
            quality: ExtractionQuality::Complete,
        }
    }

    /// Create a new ExtractedText with partial extraction.
    pub fn partial(content: String, reason: String) -> Self {
        Self {
            content,
            metadata: None,
            quality: ExtractionQuality::Partial { reason },
        }
    }
}

/// Quality indicator for text extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ExtractionQuality {
    /// All text was extracted successfully
    Complete,

    /// Some text may be missing or approximate
    Partial {
        /// Reason for partial extraction
        reason: String,
    },
}

/// Document metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// Document title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Document author
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Document subject
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,

    /// Creation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<OffsetDateTime>,

    /// Last modified timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<OffsetDateTime>,

    /// Application that created the document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator: Option<String>,
}

/// Trait for document text extractors.
pub trait Extractor {
    /// Extract text from a file path.
    fn extract_file(&self, path: &Path) -> Result<ExtractedText>;

    /// Extract text from bytes.
    fn extract_bytes(&self, bytes: &[u8]) -> Result<ExtractedText>;

    /// Returns the format name this extractor handles.
    fn format_name(&self) -> &'static str;

    /// Returns file extensions this extractor handles.
    fn extensions(&self) -> &'static [&'static str];
}

/// Options for text extraction.
#[derive(Debug, Clone, Default)]
pub struct ExtractOptions {
    /// Include document metadata in output
    pub include_metadata: bool,

    /// Maximum output size in bytes (0 = unlimited)
    pub max_output_size: usize,

    /// Timeout for extraction in milliseconds (0 = unlimited)
    pub timeout_ms: u64,
}

/// Resource limits for extraction.
///
/// These defaults are intentionally conservative since docprims parses untrusted input.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ExtractLimits {
    pub max_input_bytes: usize,
    pub max_output_bytes: usize,
    pub max_blocks: usize,
}

impl Default for ExtractLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 100 * 1024 * 1024,
            max_output_bytes: 50 * 1024 * 1024,
            max_blocks: 10_000,
        }
    }
}

/// v0 structured extraction contract.
///
/// This is the machine-consumable output shape for CLI/FFI JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocprimsExtract {
    pub schema_id: String,
    pub schema_version: String,
    pub generator: DocprimsGenerator,
    pub source: DocprimsSource,
    pub document: DocprimsDocument,
}

impl DocprimsExtract {
    pub fn v0(
        generator: DocprimsGenerator,
        source: DocprimsSource,
        document: DocprimsDocument,
    ) -> Self {
        Self {
            schema_id: DOCPRIMS_V0_EXTRACT_SCHEMA_ID.to_string(),
            schema_version: DOCPRIMS_V0_SCHEMA_VERSION.to_string(),
            generator,
            source,
            document,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocprimsGenerator {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocprimsSource {
    pub uri: String,
    pub format: DocprimsFormat,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocprimsFormat {
    pub family: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocprimsDocument {
    pub quality: DocprimsQuality,
    pub text: String,
    pub blocks: Vec<DocprimsBlock>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocprimsQuality {
    pub status: DocprimsQualityStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl DocprimsQuality {
    pub fn complete() -> Self {
        Self {
            status: DocprimsQualityStatus::Complete,
            reason: None,
        }
    }

    pub fn partial(reason: impl Into<String>) -> Self {
        Self {
            status: DocprimsQualityStatus::Partial,
            reason: Some(reason.into()),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocprimsQualityStatus {
    Complete,
    Partial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocprimsBlock {
    pub id: String,
    pub kind: String,
    pub text: String,
    pub doc_text_range: DocprimsByteRange,
    pub loc: DocprimsLocation,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub children: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DocprimsByteRange {
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocprimsLocation {
    pub kind: String,
    pub container: DocprimsContainer,
    #[serde(skip_serializing_if = "serde_json::Map::is_empty", default)]
    pub hints: serde_json::Map<String, JsonValue>,
}

impl DocprimsLocation {
    pub fn file(kind: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            container: DocprimsContainer {
                kind: DocprimsContainerKind::File,
                path: path.into(),
                part: None,
            },
            hints: serde_json::Map::new(),
        }
    }

    pub fn archive(
        kind: impl Into<String>,
        path: impl Into<String>,
        part: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            container: DocprimsContainer {
                kind: DocprimsContainerKind::Archive,
                path: path.into(),
                part: Some(part.into()),
            },
            hints: serde_json::Map::new(),
        }
    }

    pub fn with_hint_u64(mut self, key: &str, value: u64) -> Self {
        self.hints.insert(
            key.to_string(),
            JsonValue::Number(serde_json::Number::from(value)),
        );
        self
    }

    pub fn with_hint_str(mut self, key: &str, value: impl Into<String>) -> Self {
        self.hints
            .insert(key.to_string(), JsonValue::String(value.into()));
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocprimsContainer {
    pub kind: DocprimsContainerKind,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocprimsContainerKind {
    File,
    Archive,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extracted_text_complete() {
        let text = ExtractedText::complete("Hello, world!".to_string());
        assert_eq!(text.content, "Hello, world!");
        assert!(matches!(text.quality, ExtractionQuality::Complete));
    }

    #[test]
    fn test_extracted_text_partial() {
        let text = ExtractedText::partial(
            "Partial content".to_string(),
            "Some elements unsupported".to_string(),
        );
        assert!(matches!(text.quality, ExtractionQuality::Partial { .. }));
    }

    #[test]
    fn docprims_extract_v0_has_expected_ids() {
        let extract = DocprimsExtract::v0(
            DocprimsGenerator {
                name: "docprims".to_string(),
                version: "0.1.0".to_string(),
            },
            DocprimsSource {
                uri: "./test.md".to_string(),
                format: DocprimsFormat {
                    family: "text".to_string(),
                    kind: "markdown".to_string(),
                },
                sha256: None,
            },
            DocprimsDocument {
                quality: DocprimsQuality::complete(),
                text: "hi".to_string(),
                blocks: vec![DocprimsBlock {
                    id: "markdown:paragraph:0".to_string(),
                    kind: "markdown:paragraph".to_string(),
                    text: "hi".to_string(),
                    doc_text_range: DocprimsByteRange {
                        start_byte: 0,
                        end_byte: 2,
                    },
                    loc: DocprimsLocation::file("markdown:locator", "./test.md")
                        .with_hint_u64("block_index", 0),
                    children: vec![],
                    role: None,
                }],
                warnings: vec![],
                metadata: None,
            },
        );

        let v = serde_json::to_value(&extract).unwrap();
        assert_eq!(
            v.get("schema_id").unwrap().as_str().unwrap(),
            DOCPRIMS_V0_EXTRACT_SCHEMA_ID
        );
        assert_eq!(
            v.get("schema_version").unwrap().as_str().unwrap(),
            DOCPRIMS_V0_SCHEMA_VERSION
        );
    }
}
