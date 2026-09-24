//! Extracted text contains only XML 1.0 characters and no DEL or C1 controls,
//! on every format.
//!
//! Numeric character references resolve only if they name an XML 1.0 `Char`
//! (shared by the XML and OOXML extractors). Separately, every extractor drops
//! non-XML characters, DEL and C1 controls from its text before block byte ranges are computed, so
//! literal control bytes and HTML/Markdown references to them never reach the
//! output and provenance ranges still slice the text exactly.

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
    // XML Chars that are DEL or C1 controls: resolved, then dropped.
    ("&#x7F;", ""),
    ("&#x85;", ""),
    ("&#x9B;", ""),
    ("&#xA0;", "\u{a0}"),
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

#[test]
fn literal_control_characters_are_dropped_on_xml_paths() {
    let body = "a\u{1}b\u{1b}c";
    assert_eq!(via_xml(body), "abc");
    assert_eq!(via_docx(body), "abc");
}

// Markdown and HTML decode references per the HTML spec (NUL becomes U+FFFD,
// which is an XML character and is kept); the final-text filter then drops
// anything outside the XML character set.
#[test]
fn markdown_and_html_output_excludes_control_characters() {
    for (reference, expected) in [
        ("&#0;", "\u{fffd}"),
        ("&#1;", ""),
        ("&#x1B;", ""),
        ("&#9;", "\t"),
        ("&#xFFFE;", ""),
        ("&#233;", "\u{e9}"),
        ("&#x7F;", ""),
        ("&#x81;", ""),
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
    // HTML maps references in U+0080-U+009F to Windows-1252 characters where
    // one is defined ("€", "…", "›"); Markdown does not, so they are C1
    // controls there and are dropped. Either way no C1 control is emitted.
    for (reference, html_expected) in [
        ("&#x80;", "\u{20ac}"),
        ("&#x85;", "\u{2026}"),
        ("&#x9B;", "\u{203a}"),
    ] {
        let body = format!("a{reference}b");
        assert_eq!(
            via_markdown(&body).trim_end(),
            "ab",
            "markdown: {reference}"
        );
        assert_eq!(
            via_html(&body).trim_end(),
            format!("a{html_expected}b"),
            "html: {reference}"
        );
    }
    assert_eq!(
        via_markdown("a\u{1b}[31mred\u{9b}0m\n").trim_end(),
        "a[31mred0m"
    );
    assert_eq!(via_html("a\u{1b}[31mred\u{9b}0m").trim_end(), "a[31mred0m");
}

fn zip_parts(parts: &[(&str, &str)]) -> Vec<u8> {
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, body) in parts {
        w.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        w.write_all(body.as_bytes()).unwrap();
    }
    w.finish().unwrap().into_inner()
}

/// Every block's byte range slices the document text to exactly the block's
/// text, and no non-XML character survives anywhere.
fn assert_clean_and_consistent(label: &str, extract: &docprims_core::DocprimsExtract) {
    let doc = &extract.document.text;
    assert!(
        doc.chars().all(docprims_core::xml::is_output_char),
        "{label}: disallowed char in {doc:?}"
    );
    assert!(!extract.document.blocks.is_empty(), "{label}: no blocks");
    for block in &extract.document.blocks {
        let r = &block.doc_text_range;
        let slice = doc
            .get(r.start_byte..r.end_byte)
            .unwrap_or_else(|| panic!("{label}: range {r:?} invalid for {doc:?}"));
        assert_eq!(slice, block.text, "{label}: block {}", block.id);
        assert!(!block.text.is_empty(), "{label}: empty block {}", block.id);
    }
}

#[test]
fn block_ranges_slice_filtered_text_on_every_format() {
    let l = ExtractLimits::default;
    // Each input has a clean block, a block with control bytes mid-text, and a
    // block made only of control bytes (which must vanish, not leave a gap).
    let md = "first\n\nmid\u{1}d\u{7f}l\u{85}\u{9b}e \u{1b}[0m\n\n\u{7}\u{8}\n\nlast\n";
    let e = docprims_text::markdown::extract_v0_str(md, "m.md", l()).unwrap();
    assert_clean_and_consistent("markdown", &e);
    assert_eq!(e.document.text, "first\nmiddle [0m\nlast");

    let html =
        "<p>first</p><p>mid\u{1}d\u{7f}l\u{85}\u{9b}e <b>\u{1b}</b>bold</p><p>\u{7}</p><p>last</p>";
    let e = docprims_text::html::extract_v0_str(html, "h.html", l()).unwrap();
    assert_clean_and_consistent("html", &e);
    assert_eq!(e.document.text, "first\nmiddle bold\nlast");

    let xml =
        "<r><a>first</a><b>mid\u{1}d\u{7f}l\u{85}\u{9b}e&#x1B;</b><c>\u{7}</c><d>last</d></r>";
    let e = docprims_text::xml::extract_v0_str(xml, "x.xml", l()).unwrap();
    assert_clean_and_consistent("xml", &e);
    assert_eq!(e.document.text, "first middle last");

    let p = |t: &str| format!(r#"<w:p><w:r><w:t xml:space="preserve">{t}</w:t></w:r></w:p>"#);
    let docx = zip_parts(&[(
        "word/document.xml",
        &format!(
            "<w:document {W_NS}><w:body>{}{}{}{}</w:body></w:document>",
            p("first"),
            p("mid\u{1}d\u{7f}l\u{85}\u{9b}e "),
            p("\u{7}"),
            p("last")
        ),
    )]);
    let e = docprims_ooxml::extract_docx_v0_reader(Cursor::new(docx), "d.docx", l()).unwrap();
    assert_clean_and_consistent("docx", &e);
    assert_eq!(e.document.text, "first\nmiddle\nlast");

    const S: &str = r#"xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main""#;
    const R: &str =
        r#"xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships""#;
    let xlsx = zip_parts(&[
        (
            "xl/workbook.xml",
            &format!(
                r#"<workbook {S} {R}><sheets><sheet name="S" sheetId="1" r:id="rId1"/></sheets></workbook>"#
            ),
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/sharedStrings.xml",
            &format!(
                "<sst {S}><si><t>mid\u{1}d\u{7f}l\u{85}\u{9b}e</t></si><si><t>\u{7}</t></si></sst>"
            ),
        ),
        (
            "xl/worksheets/sheet1.xml",
            &format!(
                r#"<worksheet {S}><sheetData><row r="1"><c t="inlineStr"><is><t>first</t></is></c></row><row r="2"><c t="s"><v>0</v></c></row><row r="3"><c t="s"><v>1</v></c></row><row r="4"><c t="inlineStr"><is><t>last</t></is></c></row></sheetData></worksheet>"#
            ),
        ),
    ]);
    let e = docprims_ooxml::extract_xlsx_v0_reader(Cursor::new(xlsx), "s.xlsx", l()).unwrap();
    assert_clean_and_consistent("xlsx", &e);
    assert_eq!(e.document.text, "first\nmiddle\nlast");

    const P: &str = r#"xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main""#;
    let ap = |t: &str| format!("<a:p><a:r><a:t>{t}</a:t></a:r></a:p>");
    let pptx = zip_parts(&[
        ("ppt/presentation.xml", &format!(r#"<p:presentation {P} {R}><p:sldIdLst><p:sldId id="256" r:id="rId1"/></p:sldIdLst></p:presentation>"#)),
        ("ppt/_rels/presentation.xml.rels", r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/></Relationships>"#),
        ("ppt/slides/slide1.xml", &format!("<p:sld {P}><p:cSld><p:spTree><p:sp><p:txBody>{}{}{}{}</p:txBody></p:sp></p:spTree></p:cSld></p:sld>", ap("first"), ap("mid\u{1}d\u{7f}l\u{85}\u{9b}e"), ap("\u{7}"), ap("last"))),
    ]);
    let e = docprims_ooxml::extract_pptx_v0_reader(Cursor::new(pptx), "p.pptx", l()).unwrap();
    assert_clean_and_consistent("pptx", &e);
    assert_eq!(e.document.text, "first\nmiddle\nlast");
}

#[test]
fn legacy_extract_api_output_is_filtered_too() {
    let out = docprims_text::xml::extract("<r>a\u{1}b</r>").unwrap();
    assert_eq!(out.content, "ab");
    let out = docprims_text::markdown::extract("a\u{1b}b\n").unwrap();
    assert!(!out.content.contains('\u{1b}'), "{:?}", out.content);
}
