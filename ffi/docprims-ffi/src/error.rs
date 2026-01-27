use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Error codes returned by docprims FFI functions.
///
/// These are intentionally coarse; callers should read `docprims_last_error()`
/// for a human-readable message.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocprimsErrorCode {
    /// Success.
    Ok = 0,
    /// Invalid arguments / caller usage error.
    Usage = 64,
    /// Input is malformed or unsupported.
    DataInvalid = 60,
    /// Resource limit exceeded.
    ResourceLimit = 70,
    /// I/O failure reading input.
    Io = 74,
    /// Internal failure.
    Internal = 1,
}

thread_local! {
    static LAST_ERROR: RefCell<(DocprimsErrorCode, String)> = const { RefCell::new((DocprimsErrorCode::Ok, String::new())) };
}

pub fn set_last_error(code: DocprimsErrorCode, msg: impl Into<String>) {
    let msg = msg.into();
    LAST_ERROR.with(|e| *e.borrow_mut() = (code, msg));
}

pub fn clear_last_error() {
    set_last_error(DocprimsErrorCode::Ok, "");
}

pub fn last_error_code() -> DocprimsErrorCode {
    LAST_ERROR.with(|e| e.borrow().0)
}

pub fn last_error_message() -> String {
    LAST_ERROR.with(|e| e.borrow().1.clone())
}

/// Clear thread-local error state.
#[no_mangle]
pub extern "C" fn docprims_clear_error() {
    clear_last_error();
}

/// Get the last error code for the current thread.
#[no_mangle]
pub extern "C" fn docprims_last_error_code() -> DocprimsErrorCode {
    last_error_code()
}

/// Get the last error message for the current thread.
///
/// # Safety
///
/// The returned pointer must be freed with `docprims_free_string()`.
#[no_mangle]
pub extern "C" fn docprims_last_error() -> *mut c_char {
    let msg = last_error_message();
    CString::new(msg)
        .unwrap_or_else(|_| CString::new("error message contains null byte").unwrap())
        .into_raw()
}

/// Read a nullable C string parameter.
///
/// # Safety
///
/// If `ptr` is non-null it must be a valid NUL-terminated UTF-8 string.
pub unsafe fn read_opt_cstr(ptr: *const c_char) -> Result<Option<String>, DocprimsErrorCode> {
    if ptr.is_null() {
        return Ok(None);
    }
    let s = CStr::from_ptr(ptr)
        .to_str()
        .map_err(|_| DocprimsErrorCode::Usage)?;
    Ok(Some(s.to_string()))
}
