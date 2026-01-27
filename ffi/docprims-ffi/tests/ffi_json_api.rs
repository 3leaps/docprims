use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use std::ffi::{CStr, CString};
use std::path::{Path, PathBuf};

use docprims_ffi::{
    docprims_abi_version, docprims_clear_error, docprims_extract_bytes_json,
    docprims_extract_file_json, docprims_free_string, docprims_last_error,
    docprims_last_error_code, docprims_version, DocprimsErrorCode,
};

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points at ffi/docprims-ffi; repo root is two levels up.
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn write_temp_file(name: &str, bytes: &[u8]) -> PathBuf {
    let mut p = std::env::temp_dir();
    let uniq = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    // Keep `name` as the suffix so file extension-based detection works.
    p.push(format!("docprims-{uniq}-{name}"));
    std::fs::write(&p, bytes).unwrap();
    p
}

unsafe fn read_and_free(ptr: *mut std::os::raw::c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let s = CStr::from_ptr(ptr).to_string_lossy().to_string();
    docprims_free_string(ptr);
    s
}

#[test]
fn ffi_version_and_abi_version() {
    let p1 = docprims_version();
    let p2 = docprims_version();
    assert_eq!(p1, p2);
    assert!(!p1.is_null());

    let v = unsafe { CStr::from_ptr(p1).to_str().unwrap() };
    assert!(v.contains('.'));
    assert!(docprims_abi_version() > 0);
}

#[test]
fn ffi_extract_file_json_success_and_schema_id_present() {
    docprims_clear_error();

    let src = repo_root().join("testdata/fixtures/text/simple.md");
    let path_c = CString::new(src.to_string_lossy().to_string()).unwrap();
    let mut out: *mut std::os::raw::c_char = std::ptr::null_mut();

    let code = unsafe { docprims_extract_file_json(path_c.as_ptr(), std::ptr::null(), &mut out) };
    assert_eq!(code, DocprimsErrorCode::Ok);
    assert!(!out.is_null());

    let json = unsafe { read_and_free(out) };
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        v.get("schema_id").and_then(|v| v.as_str()),
        Some(docprims_core::DOCPRIMS_V0_EXTRACT_SCHEMA_ID)
    );
    assert_eq!(
        v.get("schema_version").and_then(|v| v.as_str()),
        Some(docprims_core::DOCPRIMS_V0_SCHEMA_VERSION)
    );
}

#[test]
fn ffi_extract_file_json_out_json_null_sets_last_error() {
    docprims_clear_error();

    let p = CString::new("/tmp/does-not-matter").unwrap();
    let code =
        unsafe { docprims_extract_file_json(p.as_ptr(), std::ptr::null(), std::ptr::null_mut()) };
    assert_eq!(code, DocprimsErrorCode::Usage);
    assert_eq!(docprims_last_error_code(), DocprimsErrorCode::Usage);

    let msg = unsafe { read_and_free(docprims_last_error()) };
    assert!(msg.contains("out_json"));
    docprims_clear_error();
    assert_eq!(docprims_last_error_code(), DocprimsErrorCode::Ok);
}

#[test]
fn ffi_extract_file_json_invalid_utf8_options_sets_last_error() {
    docprims_clear_error();

    let src = repo_root().join("testdata/fixtures/text/simple.md");
    let path_c = CString::new(src.to_string_lossy().to_string()).unwrap();
    // Invalid UTF-8 (NUL-terminated)
    let bad = [0xFFu8, 0x00u8];
    let bad_ptr = bad.as_ptr() as *const std::os::raw::c_char;
    let mut out: *mut std::os::raw::c_char = std::ptr::null_mut();

    let code = unsafe { docprims_extract_file_json(path_c.as_ptr(), bad_ptr, &mut out) };
    assert_eq!(code, DocprimsErrorCode::Usage);
    assert!(out.is_null());
    assert_eq!(docprims_last_error_code(), DocprimsErrorCode::Usage);
    let msg = unsafe { read_and_free(docprims_last_error()) };
    assert!(msg.contains("options_json") || msg.contains("UTF-8"));
}

#[test]
fn ffi_extract_file_json_thread_local_error_is_isolated() {
    docprims_clear_error();

    let handle = std::thread::spawn(|| {
        let p = CString::new("/tmp/does-not-matter").unwrap();
        let code = unsafe {
            docprims_extract_file_json(p.as_ptr(), std::ptr::null(), std::ptr::null_mut())
        };
        assert_eq!(code, DocprimsErrorCode::Usage);
        assert_eq!(docprims_last_error_code(), DocprimsErrorCode::Usage);
    });
    handle.join().unwrap();

    // Main thread should still be clear.
    assert_eq!(docprims_last_error_code(), DocprimsErrorCode::Ok);
}

#[test]
fn ffi_extract_file_json_parses_limits_options() {
    docprims_clear_error();

    let src = repo_root().join("testdata/fixtures/text/simple.md");
    let path_c = CString::new(src.to_string_lossy().to_string()).unwrap();
    let opts = CString::new(r#"{"limits":{"max_output_bytes":1}}"#).unwrap();
    let mut out: *mut std::os::raw::c_char = std::ptr::null_mut();

    let code = unsafe { docprims_extract_file_json(path_c.as_ptr(), opts.as_ptr(), &mut out) };
    assert_eq!(code, DocprimsErrorCode::Ok);
    let json = unsafe { read_and_free(out) };
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        v.pointer("/document/quality/status")
            .and_then(|v| v.as_str()),
        Some("partial")
    );
}

#[test]
fn ffi_extract_file_json_ooxml_file_works_with_base64_fixture() {
    docprims_clear_error();

    let b64 =
        std::fs::read_to_string(repo_root().join("testdata/fixtures/ooxml/mini.docx.b64")).unwrap();
    let compact: String = b64.lines().map(|l| l.trim()).collect();
    let bytes = STANDARD.decode(compact.as_bytes()).unwrap();
    let docx_path = write_temp_file("mini.docx", &bytes);

    let path_c = CString::new(docx_path.to_string_lossy().to_string()).unwrap();
    let mut out: *mut std::os::raw::c_char = std::ptr::null_mut();
    let code = unsafe { docprims_extract_file_json(path_c.as_ptr(), std::ptr::null(), &mut out) };
    assert_eq!(code, DocprimsErrorCode::Ok);
    let json = unsafe { read_and_free(out) };
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        v.pointer("/source/format/kind").and_then(|v| v.as_str()),
        Some("docx")
    );

    let _ = std::fs::remove_file(docx_path);
}

#[test]
fn ffi_extract_bytes_json_markdown_success() {
    docprims_clear_error();

    let data = b"# H\n\npara\n";
    let src = CString::new("mem://input.md").unwrap();
    let mut out: *mut std::os::raw::c_char = std::ptr::null_mut();
    let code = unsafe {
        docprims_extract_bytes_json(
            data.as_ptr(),
            data.len(),
            src.as_ptr(),
            std::ptr::null(),
            &mut out,
        )
    };
    assert_eq!(code, DocprimsErrorCode::Ok);
    let json = unsafe { read_and_free(out) };
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        v.pointer("/source/uri").and_then(|v| v.as_str()),
        Some("mem://input.md")
    );
    assert_eq!(
        v.pointer("/source/format/kind").and_then(|v| v.as_str()),
        Some("markdown")
    );
}

#[test]
fn ffi_extract_bytes_json_requires_source_uri_extension() {
    docprims_clear_error();

    let data = b"hi";
    let src = CString::new("mem://input").unwrap();
    let mut out: *mut std::os::raw::c_char = std::ptr::null_mut();
    let code = unsafe {
        docprims_extract_bytes_json(
            data.as_ptr(),
            data.len(),
            src.as_ptr(),
            std::ptr::null(),
            &mut out,
        )
    };

    assert_eq!(code, DocprimsErrorCode::DataInvalid);
    assert!(out.is_null());
    assert_eq!(docprims_last_error_code(), DocprimsErrorCode::DataInvalid);
}

#[test]
fn ffi_extract_bytes_json_ooxml_success_from_base64_fixture() {
    docprims_clear_error();

    let b64 =
        std::fs::read_to_string(repo_root().join("testdata/fixtures/ooxml/mini.docx.b64")).unwrap();
    let compact: String = b64.lines().map(|l| l.trim()).collect();
    let bytes = STANDARD.decode(compact.as_bytes()).unwrap();
    let src = CString::new("mem://upload.docx").unwrap();

    let mut out: *mut std::os::raw::c_char = std::ptr::null_mut();
    let code = unsafe {
        docprims_extract_bytes_json(
            bytes.as_ptr(),
            bytes.len(),
            src.as_ptr(),
            std::ptr::null(),
            &mut out,
        )
    };
    assert_eq!(code, DocprimsErrorCode::Ok);
    let json = unsafe { read_and_free(out) };
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        v.pointer("/source/format/kind").and_then(|v| v.as_str()),
        Some("docx")
    );
}
