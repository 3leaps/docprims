use std::path::Path;

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;

use docprims_core::{DocprimsError, ExtractLimits};

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocprimsErrorCode {
    Ok = 0,
    Usage = 64,
    DataInvalid = 60,
    ResourceLimit = 70,
    Io = 74,
    Internal = 1,
}

fn map_err(e: &DocprimsError) -> DocprimsErrorCode {
    match e {
        DocprimsError::UnknownFormat(_) => DocprimsErrorCode::DataInvalid,
        DocprimsError::Malformed(_) => DocprimsErrorCode::DataInvalid,
        DocprimsError::Parse(_) => DocprimsErrorCode::DataInvalid,
        DocprimsError::ResourceLimit(_) => DocprimsErrorCode::ResourceLimit,
        DocprimsError::Io(_) => DocprimsErrorCode::Io,
        _ => DocprimsErrorCode::Internal,
    }
}

#[napi(object)]
pub struct DocprimsCallJsonResult {
    pub code: i32,
    pub json: Option<String>,
    pub message: Option<String>,
}

fn ok_json(json: String) -> DocprimsCallJsonResult {
    DocprimsCallJsonResult {
        code: DocprimsErrorCode::Ok as i32,
        json: Some(json),
        message: None,
    }
}

fn err_json(code: DocprimsErrorCode, msg: impl Into<String>) -> DocprimsCallJsonResult {
    DocprimsCallJsonResult {
        code: code as i32,
        json: None,
        message: Some(msg.into()),
    }
}

#[derive(Debug, serde::Deserialize)]
struct DocprimsExtractOptions {
    #[serde(default)]
    limits: Option<ExtractLimitsOverrides>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct ExtractLimitsOverrides {
    #[serde(default)]
    max_input_bytes: Option<usize>,
    #[serde(default)]
    max_output_bytes: Option<usize>,
    #[serde(default)]
    max_blocks: Option<usize>,
}

fn parse_limits(options_json: &str) -> Result<ExtractLimits, DocprimsErrorCode> {
    if options_json.trim().is_empty() {
        return Ok(ExtractLimits::default());
    }

    let opts: DocprimsExtractOptions =
        serde_json::from_str(options_json).map_err(|_| DocprimsErrorCode::Usage)?;

    let mut limits = ExtractLimits::default();
    if let Some(overrides) = opts.limits {
        if let Some(v) = overrides.max_input_bytes {
            limits.max_input_bytes = v;
        }
        if let Some(v) = overrides.max_output_bytes {
            limits.max_output_bytes = v;
        }
        if let Some(v) = overrides.max_blocks {
            limits.max_blocks = v;
        }
    }
    Ok(limits)
}

fn ext_lowercase(s: &str) -> String {
    Path::new(s)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default()
}

#[napi]
pub fn docprims_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[napi]
pub fn docprims_abi_version() -> u32 {
    1
}

#[napi]
pub fn docprims_extract_file_json(path: String, options_json: String) -> DocprimsCallJsonResult {
    let limits = match parse_limits(&options_json) {
        Ok(l) => l,
        Err(code) => return err_json(code, "invalid options_json"),
    };

    let ext = ext_lowercase(&path);
    let extracted = match ext.as_str() {
        "docx" => docprims_ooxml::extract_docx_v0(&path, limits),
        "xlsx" => docprims_ooxml::extract_xlsx_v0(&path, limits),
        "pptx" => docprims_ooxml::extract_pptx_v0(&path, limits),
        "md" | "markdown" => docprims_text::extract_markdown_v0(&path, limits),
        "html" | "htm" => docprims_text::extract_html_v0(&path, limits),
        "xml" => docprims_text::extract_xml_v0(&path, limits),
        _ => Err(DocprimsError::UnknownFormat(ext)),
    };

    match extracted {
        Ok(extract) => match serde_json::to_string(&extract) {
            Ok(s) => ok_json(s),
            Err(e) => err_json(
                DocprimsErrorCode::Internal,
                format!("error serializing result: {e}"),
            ),
        },
        Err(e) => {
            let code = map_err(&e);
            err_json(code, e.to_string())
        }
    }
}

#[napi]
pub fn docprims_extract_bytes_json(
    source_uri: String,
    data: Buffer,
    options_json: String,
) -> DocprimsCallJsonResult {
    let limits = match parse_limits(&options_json) {
        Ok(l) => l,
        Err(code) => return err_json(code, "invalid options_json"),
    };

    if data.len() > limits.max_input_bytes {
        return err_json(
            DocprimsErrorCode::ResourceLimit,
            format!(
                "input exceeds max_input_bytes ({} > {})",
                data.len(),
                limits.max_input_bytes
            ),
        );
    }

    let ext = ext_lowercase(&source_uri);
    if ext.is_empty() {
        return err_json(
            DocprimsErrorCode::DataInvalid,
            "unsupported format: missing extension in source_uri",
        );
    }

    let bytes = data.as_ref();
    let extracted = match ext.as_str() {
        "docx" => {
            docprims_ooxml::extract_docx_v0_reader(std::io::Cursor::new(bytes), &source_uri, limits)
        }
        "xlsx" => {
            docprims_ooxml::extract_xlsx_v0_reader(std::io::Cursor::new(bytes), &source_uri, limits)
        }
        "pptx" => {
            docprims_ooxml::extract_pptx_v0_reader(std::io::Cursor::new(bytes), &source_uri, limits)
        }
        "md" | "markdown" => match std::str::from_utf8(bytes) {
            Ok(s) => docprims_text::markdown::extract_v0_str(s, &source_uri, limits),
            Err(_) => Err(DocprimsError::Malformed(
                "non-utf8 markdown input".to_string(),
            )),
        },
        "html" | "htm" => match std::str::from_utf8(bytes) {
            Ok(s) => docprims_text::html::extract_v0_str(s, &source_uri, limits),
            Err(_) => Err(DocprimsError::Malformed("non-utf8 html input".to_string())),
        },
        "xml" => match std::str::from_utf8(bytes) {
            Ok(s) => docprims_text::xml::extract_v0_str(s, &source_uri, limits),
            Err(_) => Err(DocprimsError::Malformed("non-utf8 xml input".to_string())),
        },
        _ => Err(DocprimsError::UnknownFormat(ext)),
    };

    match extracted {
        Ok(extract) => match serde_json::to_string(&extract) {
            Ok(s) => ok_json(s),
            Err(e) => err_json(
                DocprimsErrorCode::Internal,
                format!("error serializing result: {e}"),
            ),
        },
        Err(e) => {
            let code = map_err(&e);
            err_json(code, e.to_string())
        }
    }
}
