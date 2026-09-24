//! The caller's declared format picks the parser; content never does.

use std::io::{Cursor, Write};

use docprims::{DocprimsError, ExtractLimits, Format};

fn docx_bytes() -> Vec<u8> {
    let doc = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>inside docx</w:t></w:r></w:p></w:body></w:document>"#;
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file(
        "word/document.xml",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    w.write_all(doc.as_bytes()).unwrap();
    w.finish().unwrap().into_inner()
}

#[test]
fn docx_payload_named_markdown_goes_to_the_markdown_parser() {
    match docprims::extract_bytes("upload.md", &docx_bytes(), ExtractLimits::default()) {
        Err(DocprimsError::Malformed(msg)) => assert!(msg.contains("markdown"), "{msg}"),
        Ok(out) => {
            assert_eq!(out.source.format.kind, "markdown");
            assert!(!out.document.text.contains("inside docx"));
        }
        Err(other) => panic!("unexpected error {other:?}"),
    }
}

#[test]
fn markdown_named_docx_goes_to_the_ooxml_parser() {
    let err = docprims::extract_bytes("notes.docx", b"# heading\n", ExtractLimits::default())
        .unwrap_err();
    assert!(matches!(err, DocprimsError::Malformed(_)), "{err:?}");
}

#[test]
fn explicit_format_overrides_the_name() {
    let out = docprims::extract_bytes_as(
        Format::Docx,
        "upload.bin",
        &docx_bytes(),
        ExtractLimits::default(),
    )
    .unwrap();
    assert_eq!(out.source.format.kind, "docx");
    assert_eq!(out.source.uri, "upload.bin");
    assert_eq!(out.document.text, "inside docx");

    let out = docprims::extract_bytes_as(
        Format::Markdown,
        "notes.docx",
        b"# heading\n",
        ExtractLimits::default(),
    )
    .unwrap();
    assert_eq!(out.source.format.kind, "markdown");
}

#[test]
fn file_and_bytes_paths_agree() {
    let dir = std::env::temp_dir().join(format!("docprims-dispatch-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("doc.docx");
    std::fs::write(&path, docx_bytes()).unwrap();
    let from_file = docprims::extract_file(&path, ExtractLimits::default()).unwrap();
    let from_bytes = docprims::extract_bytes(
        path.to_str().unwrap(),
        &docx_bytes(),
        ExtractLimits::default(),
    )
    .unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(
        serde_json::to_value(&from_file.document).unwrap(),
        serde_json::to_value(&from_bytes.document).unwrap()
    );
}

#[test]
fn oversized_input_is_rejected_before_parsing() {
    let limits = ExtractLimits {
        max_input_bytes: 4,
        ..ExtractLimits::default()
    };
    let err = docprims::extract_bytes("a.md", b"# heading", limits).unwrap_err();
    assert!(matches!(err, DocprimsError::ResourceLimit(_)), "{err:?}");
}
