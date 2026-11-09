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

    let extracted = docprims::extract_file(&path, limits);

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

    let extracted = docprims::extract_bytes(&source_uri, data.as_ref(), limits);

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
