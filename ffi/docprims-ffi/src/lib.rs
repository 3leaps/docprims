//! # docprims-ffi
//!
//! C-ABI FFI for docprims document text extraction.
//!
//! This crate provides C-compatible functions for use from Go, Python,
//! TypeScript/Node, and other languages via FFI.
//!
//! ## Memory Management
//!
//! All strings returned by this library must be freed using `docprims_free_string()`.
//! The caller is responsible for freeing the memory.
//!
//! ## Thread Safety
//!
//! All functions are thread-safe and can be called from multiple threads.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;
use std::ptr;

/// Result structure returned by extraction functions.
#[repr(C)]
pub struct DocprimsResult {
    /// Extracted text content (NULL on error)
    pub content: *mut c_char,
    /// JSON-encoded metadata (NULL if not requested or unavailable)
    pub metadata_json: *mut c_char,
    /// Error message (NULL on success)
    pub error: *mut c_char,
}

/// Extract text from a file path.
///
/// # Safety
///
/// - `path` must be a valid null-terminated UTF-8 string
/// - The returned `DocprimsResult` must be freed with `docprims_free_result()`
#[no_mangle]
pub unsafe extern "C" fn docprims_extract_file(path: *const c_char) -> *mut DocprimsResult {
    let result = Box::new(DocprimsResult {
        content: ptr::null_mut(),
        metadata_json: ptr::null_mut(),
        error: ptr::null_mut(),
    });

    if path.is_null() {
        return Box::into_raw(set_error(result, "path is null"));
    }

    let path_str = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return Box::into_raw(set_error(result, "invalid UTF-8 in path")),
    };

    let path = Path::new(path_str);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    let extracted = match ext.as_str() {
        "docx" => docprims_ooxml::extract_docx(path),
        "xlsx" => docprims_ooxml::extract_xlsx(path),
        "pptx" => docprims_ooxml::extract_pptx(path),
        "md" | "markdown" => docprims_text::extract_markdown(path),
        "html" | "htm" => docprims_text::extract_html(path),
        "xml" => docprims_text::extract_xml(path),
        _ => return Box::into_raw(set_error(result, &format!("unsupported format: {}", ext))),
    };

    match extracted {
        Ok(text) => {
            let mut result = result;
            result.content = match CString::new(text.content) {
                Ok(s) => s.into_raw(),
                Err(_) => return Box::into_raw(set_error(result, "content contains null byte")),
            };
            Box::into_raw(result)
        }
        Err(e) => Box::into_raw(set_error(result, &e.to_string())),
    }
}

/// Free a result structure.
///
/// # Safety
///
/// - `result` must be a pointer returned by a docprims extraction function
/// - `result` must not be used after calling this function
#[no_mangle]
pub unsafe extern "C" fn docprims_free_result(result: *mut DocprimsResult) {
    if result.is_null() {
        return;
    }

    let result = Box::from_raw(result);

    if !result.content.is_null() {
        drop(CString::from_raw(result.content));
    }
    if !result.metadata_json.is_null() {
        drop(CString::from_raw(result.metadata_json));
    }
    if !result.error.is_null() {
        drop(CString::from_raw(result.error));
    }
}

/// Free a string returned by this library.
///
/// # Safety
///
/// - `s` must be a pointer returned by a docprims function
/// - `s` must not be used after calling this function
#[no_mangle]
pub unsafe extern "C" fn docprims_free_string(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

/// Get the library version.
///
/// # Safety
///
/// The returned string is statically allocated and must not be freed.
#[no_mangle]
pub extern "C" fn docprims_version() -> *const c_char {
    static VERSION: &[u8] = concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes();
    VERSION.as_ptr() as *const c_char
}

// Helper to set error on result
fn set_error(mut result: Box<DocprimsResult>, msg: &str) -> Box<DocprimsResult> {
    result.error = CString::new(msg)
        .unwrap_or_else(|_| CString::new("error message contains null byte").unwrap())
        .into_raw();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        let version = docprims_version();
        assert!(!version.is_null());
        let version_str = unsafe { CStr::from_ptr(version).to_str().unwrap() };
        assert!(!version_str.is_empty());
    }
}
