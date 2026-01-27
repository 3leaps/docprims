//! docprims-ffi: C-ABI exports for docprims
//!
//! This crate provides a stable C-ABI interface for docprims extraction.
//!
//! # Memory Management
//!
//! All strings returned by this API must be freed with `docprims_free_string()`.
//! Do not use `free()` or any other deallocator.
//!
//! # Error Handling
//!
//! Functions return `DocprimsErrorCode`. On error, detailed information is
//! available via:
//! - `docprims_last_error_code()`
//! - `docprims_last_error()`
//! - `docprims_clear_error()`
//!
//! Error state is thread-local.
//!
//! # ABI Version
//!
//! Check `docprims_abi_version()` for ABI compatibility.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;

use docprims_core::{DocprimsError, ExtractLimits};

mod error;

pub use error::{
    docprims_clear_error, docprims_last_error, docprims_last_error_code, DocprimsErrorCode,
};

// ==========================================================================
// Version Constants
// ==========================================================================

const VERSION: &str = env!("CARGO_PKG_VERSION");
const ABI_VERSION: u32 = 1;

/// Get the library version string.
///
/// Returns a static pointer and must NOT be freed.
#[no_mangle]
pub extern "C" fn docprims_version() -> *const c_char {
    static VERSION_CSTR: std::sync::OnceLock<CString> = std::sync::OnceLock::new();
    VERSION_CSTR
        .get_or_init(|| CString::new(VERSION).expect("VERSION should not contain null bytes"))
        .as_ptr()
}

/// Get the ABI version number.
#[no_mangle]
pub extern "C" fn docprims_abi_version() -> u32 {
    ABI_VERSION
}

// ==========================================================================
// Memory Management
// ==========================================================================

/// Frees a string allocated by docprims functions.
///
/// # Safety
///
/// The pointer must have been returned by a docprims function that allocates
/// strings (e.g., `docprims_extract_file_json`, `docprims_last_error`).
/// Passing null is safe and is a no-op.
#[no_mangle]
pub unsafe extern "C" fn docprims_free_string(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    let _ = CString::from_raw(s);
}

// ==========================================================================
// Extraction (v0 JSON)
// ==========================================================================

#[derive(Debug, serde::Deserialize)]
struct DocprimsExtractOptions {
    #[serde(default)]
    limits: Option<ExtractLimits>,
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

fn parse_limits(options_json: Option<String>) -> Result<ExtractLimits, DocprimsErrorCode> {
    let Some(s) = options_json else {
        return Ok(ExtractLimits::default());
    };
    if s.trim().is_empty() {
        return Ok(ExtractLimits::default());
    }

    let opts: DocprimsExtractOptions =
        serde_json::from_str(&s).map_err(|_| DocprimsErrorCode::Usage)?;
    Ok(opts.limits.unwrap_or_default())
}

/// Extract a schema-conformant v0 `DocprimsExtract` JSON string from a file path.
///
/// # Safety
///
/// - `path` must be a valid NUL-terminated UTF-8 string
/// - `options_json` may be null; when non-null it must be UTF-8 JSON
/// - `out_json` must be a valid pointer to a `char*` slot
/// - On success, `*out_json` must be freed with `docprims_free_string()`
#[no_mangle]
pub unsafe extern "C" fn docprims_extract_file_json(
    path: *const c_char,
    options_json: *const c_char,
    out_json: *mut *mut c_char,
) -> DocprimsErrorCode {
    error::clear_last_error();

    if out_json.is_null() {
        error::set_last_error(DocprimsErrorCode::Usage, "out_json is null");
        return DocprimsErrorCode::Usage;
    }
    *out_json = std::ptr::null_mut();

    if path.is_null() {
        error::set_last_error(DocprimsErrorCode::Usage, "path is null");
        return DocprimsErrorCode::Usage;
    }

    let path_str = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => {
            error::set_last_error(DocprimsErrorCode::Usage, "invalid UTF-8 in path");
            return DocprimsErrorCode::Usage;
        }
    };

    let options = match error::read_opt_cstr(options_json) {
        Ok(o) => o,
        Err(code) => {
            error::set_last_error(code, "invalid UTF-8 in options_json");
            return code;
        }
    };

    let limits = match parse_limits(options) {
        Ok(l) => l,
        Err(code) => {
            error::set_last_error(code, "invalid options_json");
            return code;
        }
    };

    let path = Path::new(path_str);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    let extracted = match ext.as_str() {
        "docx" => docprims_ooxml::extract_docx_v0(path, limits),
        "xlsx" => docprims_ooxml::extract_xlsx_v0(path, limits),
        "pptx" => docprims_ooxml::extract_pptx_v0(path, limits),
        "md" | "markdown" => docprims_text::extract_markdown_v0(path, limits),
        "html" | "htm" => docprims_text::extract_html_v0(path, limits),
        "xml" => docprims_text::extract_xml_v0(path, limits),
        _ => Err(DocprimsError::UnknownFormat(ext)),
    };

    match extracted {
        Ok(extract) => {
            let json = match serde_json::to_string(&extract) {
                Ok(s) => s,
                Err(e) => {
                    error::set_last_error(
                        DocprimsErrorCode::Internal,
                        format!("error serializing result: {e}"),
                    );
                    return DocprimsErrorCode::Internal;
                }
            };
            match CString::new(json) {
                Ok(s) => {
                    *out_json = s.into_raw();
                    DocprimsErrorCode::Ok
                }
                Err(_) => {
                    error::set_last_error(DocprimsErrorCode::Internal, "json contains null byte");
                    DocprimsErrorCode::Internal
                }
            }
        }
        Err(e) => {
            let code = map_err(&e);
            error::set_last_error(code, e.to_string());
            code
        }
    }
}

// ==========================================================================
// Tests
// ==========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn test_version_returns_valid_string() {
        let version_ptr = docprims_version();
        assert!(!version_ptr.is_null());
        let version = unsafe { CStr::from_ptr(version_ptr).to_str().unwrap() };
        assert!(version.contains('.'));
    }

    #[test]
    fn test_version_is_stable_pointer() {
        let p1 = docprims_version();
        let p2 = docprims_version();
        assert_eq!(p1, p2);
    }

    #[test]
    fn test_abi_version_is_positive() {
        assert!(docprims_abi_version() > 0);
    }
}
