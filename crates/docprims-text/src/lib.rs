//! # docprims-text
//!
//! Text-based format extraction for docprims.
//!
//! This crate extracts text from:
//! - Markdown files
//! - HTML documents
//! - XML documents
//!
//! ## Feature Flags
//!
//! - `markdown` - Enable Markdown extraction (default)
//! - `html` - Enable HTML extraction (default)
//! - `xml` - Enable XML extraction (default)

use docprims_core::{DocprimsExtract, ExtractLimits, ExtractedText, Result};
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[cfg(feature = "markdown")]
pub mod markdown;

#[cfg(feature = "html")]
pub mod html;

#[cfg(feature = "xml")]
pub mod xml;

/// Extract text from a Markdown file.
#[cfg(feature = "markdown")]
pub fn extract_markdown(path: impl AsRef<Path>) -> Result<ExtractedText> {
    let content = std::fs::read_to_string(path.as_ref())?;
    extract_markdown_str(&content)
}

/// Extract structured output (v0 contract) from a Markdown file.
#[cfg(feature = "markdown")]
pub fn extract_markdown_v0(
    path: impl AsRef<Path>,
    limits: ExtractLimits,
) -> Result<DocprimsExtract> {
    let path = path.as_ref();
    let bytes = read_file_limited(path, limits.max_input_bytes)?;
    let s = std::str::from_utf8(&bytes).map_err(|_| {
        docprims_core::DocprimsError::Malformed("non-utf8 markdown input".to_string())
    })?;
    markdown::extract_v0_str(s, &path.display().to_string(), limits)
}

/// Extract text from a Markdown string.
#[cfg(feature = "markdown")]
pub fn extract_markdown_str(content: &str) -> Result<ExtractedText> {
    markdown::extract(content)
}

fn read_file_limited(path: &Path, max_bytes: usize) -> Result<Vec<u8>> {
    let meta = std::fs::metadata(path)?;
    if meta.len() as usize > max_bytes {
        return Err(docprims_core::DocprimsError::ResourceLimit(format!(
            "input exceeds max_input_bytes ({} > {})",
            meta.len(),
            max_bytes
        )));
    }

    let mut f = File::open(path)?;
    let mut buf = Vec::with_capacity(meta.len().min(max_bytes as u64) as usize);
    f.read_to_end(&mut buf)?;
    if buf.len() > max_bytes {
        return Err(docprims_core::DocprimsError::ResourceLimit(format!(
            "input exceeds max_input_bytes ({} > {})",
            buf.len(),
            max_bytes
        )));
    }
    Ok(buf)
}

/// Extract text from an HTML file.
#[cfg(feature = "html")]
pub fn extract_html(path: impl AsRef<Path>) -> Result<ExtractedText> {
    let content = std::fs::read_to_string(path.as_ref())?;
    extract_html_str(&content)
}

/// Extract structured output (v0 contract) from an HTML file.
#[cfg(feature = "html")]
pub fn extract_html_v0(path: impl AsRef<Path>, limits: ExtractLimits) -> Result<DocprimsExtract> {
    let path = path.as_ref();
    let bytes = read_file_limited(path, limits.max_input_bytes)?;
    let s = std::str::from_utf8(&bytes)
        .map_err(|_| docprims_core::DocprimsError::Malformed("non-utf8 html input".to_string()))?;
    html::extract_v0_str(s, &path.display().to_string(), limits)
}

/// Extract text from an HTML string.
#[cfg(feature = "html")]
pub fn extract_html_str(content: &str) -> Result<ExtractedText> {
    html::extract(content)
}

/// Extract text from an XML file.
#[cfg(feature = "xml")]
pub fn extract_xml(path: impl AsRef<Path>) -> Result<ExtractedText> {
    let content = std::fs::read_to_string(path.as_ref())?;
    extract_xml_str(&content)
}

/// Extract structured output (v0 contract) from an XML file.
#[cfg(feature = "xml")]
pub fn extract_xml_v0(path: impl AsRef<Path>, limits: ExtractLimits) -> Result<DocprimsExtract> {
    let path = path.as_ref();
    let bytes = read_file_limited(path, limits.max_input_bytes)?;
    let s = std::str::from_utf8(&bytes)
        .map_err(|_| docprims_core::DocprimsError::Malformed("non-utf8 xml input".to_string()))?;
    xml::extract_v0_str(s, &path.display().to_string(), limits)
}

/// Extract text from an XML string.
#[cfg(feature = "xml")]
pub fn extract_xml_str(content: &str) -> Result<ExtractedText> {
    xml::extract(content)
}

#[cfg(test)]
pub(crate) mod test_support {
    use docprims_core::DocprimsExtract;
    use jsonschema::Resource;

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
}
