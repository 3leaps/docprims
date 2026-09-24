//! Character references resolve identically on every XML-based path.
//!
//! The XML text extractor and the OOXML extractors share one rule: a numeric
//! character reference resolves only if it names an XML 1.0 `Char`; anything
//! else is dropped. Markdown and HTML follow their own specs and are pinned
//! here so a change to either is deliberate.

use std::io::{Cursor, Write};

use docprims_core::ExtractLimits;

const W_NS: &str = r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main""#;

fn via_xml(body: &str) -> String {
    let xml = format!("<root>{body}</root>");
    docprims_text::xml::extract_v0_str(&xml, "mem.xml", ExtractLimits::default())
        .unwrap()
        .document
        .text
}

fn via_docx(body: &str) -> String {
    let doc = format!(
        r#"<w:document {W_NS}><w:body><w:p><w:r><w:t xml:space="preserve">{body}</w:t></w:r></w:p></w:body></w:document>"#
    );
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file(
        "word/document.xml",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    w.write_all(doc.as_bytes()).unwrap();
    let bytes = w.finish().unwrap().into_inner();
    docprims_ooxml::extract_docx_v0_reader(Cursor::new(bytes), "mem.docx", ExtractLimits::default())
        .unwrap()
        .document
        .text
}

fn via_markdown(body: &str) -> String {
    docprims_text::markdown::extract_v0_str(body, "mem.md", ExtractLimits::default())
        .unwrap()
        .document
        .text
}

fn via_html(body: &str) -> String {
    let html = format!("<p>{body}</p>");
    docprims_text::html::extract_v0_str(&html, "mem.html", ExtractLimits::default())
        .unwrap()
        .document
        .text
}

/// (reference, text expected between `a` and `b` on the XML-based paths)
const CASES: &[(&str, &str)] = &[
    ("&#0;", ""),
    ("&#1;", ""),
    ("&#x1B;", ""),
    ("&#x1F;", ""),
    ("&#9;", "\t"),
    ("&#xA;", "\n"),
    ("&#xD;", "\r"),
    ("&#x20;", " "),
    ("&#xD7FF;", "\u{d7ff}"),
    ("&#xD800;", ""),
    ("&#xE000;", "\u{e000}"),
    ("&#xFFFD;", "\u{fffd}"),
    ("&#xFFFE;", ""),
    ("&#xFFFF;", ""),
    ("&#x10000;", "\u{10000}"),
    ("&#x10FFFF;", "\u{10ffff}"),
    ("&#x110000;", ""),
    ("&#233;", "\u{e9}"),
];

#[test]
fn xml_and_ooxml_resolve_character_references_identically() {
    for (reference, expected) in CASES {
        let body = format!("a{reference}b");
        let want = format!("a{expected}b");
        assert_eq!(via_xml(&body), want, "xml path: {reference}");
        assert_eq!(via_docx(&body), want, "docx path: {reference}");
    }
}

// Current behaviour, pinned. Literal (unescaped) control characters are not
// well-formed XML; the parser passes them through on both XML-based paths.
#[test]
fn literal_control_characters_pass_through_both_xml_paths() {
    let body = "a\u{1}b\u{1b}c";
    assert_eq!(via_xml(body), body);
    assert_eq!(via_docx(body), body);
}

// Current behaviour, pinned. Markdown and HTML follow the HTML numeric
// character reference rules: NUL becomes U+FFFD, other controls and
// noncharacters are passed through.
#[test]
fn markdown_and_html_character_reference_handling_is_pinned() {
    for (reference, expected) in [
        ("&#0;", "\u{fffd}"),
        ("&#1;", "\u{1}"),
        ("&#x1B;", "\u{1b}"),
        ("&#9;", "\t"),
        ("&#xFFFE;", "\u{fffe}"),
        ("&#233;", "\u{e9}"),
    ] {
        let body = format!("a{reference}b");
        let want = format!("a{expected}b");
        assert_eq!(
            via_markdown(&body).trim_end(),
            want,
            "markdown: {reference}"
        );
        assert_eq!(via_html(&body).trim_end(), want, "html: {reference}");
    }
}
