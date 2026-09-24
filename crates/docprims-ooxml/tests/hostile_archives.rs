//! Behavior of the OOXML extractors on hostile and malformed archives.
//!
//! Every fixture is generated in-test from benign content. The properties
//! checked are the ones an embedding host relies on: malformed input yields
//! an error rather than a panic, resource use stays bounded, and text is
//! neither lost nor double-decoded.

use std::io::{Cursor, Write};
use std::time::{Duration, Instant};

use docprims_core::{DocprimsError, ExtractLimits};
use docprims_ooxml::{
    extract_docx_reader, extract_docx_v0_reader, extract_pptx_reader, extract_xlsx_reader,
};
use zip::write::SimpleFileOptions;

const DECL: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#;
const W_NS: &str = r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main""#;
const S_NS: &str = r#"xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main""#;
const R_NS: &str =
    r#"xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships""#;

fn zip_of(parts: &[(&str, &str)]) -> Vec<u8> {
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, body) in parts {
        w.start_file(*name, opts).unwrap();
        w.write_all(body.as_bytes()).unwrap();
    }
    w.finish().unwrap().into_inner()
}

fn docx(body: &str) -> Vec<u8> {
    let doc = format!("{DECL}<w:document {W_NS}><w:body>{body}</w:body></w:document>");
    zip_of(&[("word/document.xml", &doc)])
}

fn xlsx(sheet_data: &str, shared: &[&str]) -> Vec<u8> {
    let workbook = format!(
        r#"{DECL}<workbook {S_NS} {R_NS}><sheets><sheet name="S1" sheetId="1" r:id="rId1"/></sheets></workbook>"#
    );
    let rels = format!(
        r#"{DECL}<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#
    );
    let sst: String = shared
        .iter()
        .map(|s| format!("<si><t>{s}</t></si>"))
        .collect();
    let sst = format!("{DECL}<sst {S_NS}>{sst}</sst>");
    let sheet = format!("{DECL}<worksheet {S_NS}><sheetData>{sheet_data}</sheetData></worksheet>");
    zip_of(&[
        ("xl/workbook.xml", &workbook),
        ("xl/_rels/workbook.xml.rels", &rels),
        ("xl/sharedStrings.xml", &sst),
        ("xl/worksheets/sheet1.xml", &sheet),
    ])
}

fn para(text: &str) -> String {
    format!(r#"<w:p><w:r><w:t xml:space="preserve">{text}</w:t></w:r></w:p>"#)
}

// --- archive-level failures: error, never panic --------------------------

#[test]
fn non_archive_inputs_are_malformed_for_every_format() {
    let good = docx(&para("hello"));
    let cases: [(&str, Vec<u8>); 3] = [
        ("empty", Vec::new()),
        ("not a zip", b"plain text, no archive here".to_vec()),
        ("truncated", good[..good.len() / 2].to_vec()),
    ];
    for (label, bytes) in cases {
        for (fmt, result) in [
            (
                "docx",
                extract_docx_reader(Cursor::new(bytes.clone())).err(),
            ),
            (
                "xlsx",
                extract_xlsx_reader(Cursor::new(bytes.clone())).err(),
            ),
            (
                "pptx",
                extract_pptx_reader(Cursor::new(bytes.clone())).err(),
            ),
        ] {
            assert!(
                matches!(result, Some(DocprimsError::Malformed(_))),
                "{fmt}/{label}: expected Malformed, got {result:?}"
            );
        }
    }
}

#[test]
fn archive_without_main_part_is_an_error() {
    let bytes = zip_of(&[("[Content_Types].xml", "<Types/>")]);
    assert!(extract_docx_reader(Cursor::new(bytes.clone())).is_err());
    assert!(extract_xlsx_reader(Cursor::new(bytes.clone())).is_err());
    assert!(extract_pptx_reader(Cursor::new(bytes)).is_err());
}

#[test]
fn archive_entry_count_is_limited() {
    let names: Vec<String> = (0..10_001).map(|i| format!("f{i}")).collect();
    let parts: Vec<(&str, &str)> = names.iter().map(|n| (n.as_str(), "")).collect();
    let err = extract_docx_reader(Cursor::new(zip_of(&parts))).unwrap_err();
    assert!(
        matches!(err, DocprimsError::ResourceLimit(_)),
        "got {err:?}"
    );
}

// --- XML-level robustness -------------------------------------------------

/// Best-of-3 extraction time for one cell carrying `n` distinct attributes.
fn time_unique_attributes(n: usize) -> Duration {
    let attrs: String = (0..n).map(|i| format!(r#" a{i}="v""#)).collect();
    let row = format!(r#"<row r="1"><c r="A1" t="s"{attrs}><v>0</v></c></row>"#);
    let bytes = xlsx(&row, &["kept"]);
    (0..3)
        .map(|_| {
            let start = Instant::now();
            let out = extract_xlsx_reader(Cursor::new(bytes.clone())).unwrap();
            let elapsed = start.elapsed();
            assert_eq!(out.content.trim(), "kept");
            elapsed
        })
        .min()
        .unwrap()
}

#[test]
fn many_unique_attributes_on_one_element_scale_linearly() {
    // Quadruple the attribute count: linear work grows ~4x, a quadratic
    // duplicate-attribute check ~16x. A ratio is used rather than an absolute
    // bound so host speed and build profile do not decide the outcome.
    let small = time_unique_attributes(25_000);
    let large = time_unique_attributes(100_000);
    let ratio = large.as_secs_f64() / small.as_secs_f64().max(1e-3);
    assert!(
        ratio < 10.0,
        "4x attributes took {ratio:.1}x as long ({small:?} -> {large:?})"
    );
    // Hang guard only; the ratio above is the real check.
    assert!(large < Duration::from_secs(60), "took {large:?}");
}

#[test]
fn deeply_nested_elements_do_not_exhaust_the_stack() {
    let depth = 200_000;
    let body = format!(
        "{}{}{}",
        "<w:sdt>".repeat(depth),
        para("deep"),
        "</w:sdt>".repeat(depth)
    );
    let out = extract_docx_reader(Cursor::new(docx(&body))).unwrap();
    assert_eq!(out.content.trim(), "deep");
}

#[test]
fn unterminated_document_xml_does_not_panic() {
    let doc = format!("{DECL}<w:document {W_NS}><w:body><w:p><w:r><w:t>cut off");
    let bytes = zip_of(&[("word/document.xml", &doc)]);
    // Either a parse error or best-effort partial text is acceptable; a panic
    // across the FFI/napi boundary is not.
    let _ = extract_docx_reader(Cursor::new(bytes));
}

#[test]
fn shared_string_index_out_of_range_does_not_panic() {
    let row = r#"<row r="1"><c r="A1" t="s"><v>99</v></c><c r="B1" t="s"><v>18446744073709551616</v></c><c r="C1" t="s"><v>-1</v></c></row>"#;
    let out = extract_xlsx_reader(Cursor::new(xlsx(row, &["only"]))).unwrap();
    assert!(
        !out.content.contains("only"),
        "unrelated shared string leaked: {:?}",
        out.content
    );
}

// --- text fidelity --------------------------------------------------------

#[test]
fn escaped_markup_is_decoded_exactly_once() {
    let out = extract_docx_reader(Cursor::new(docx(&para(
        "a &amp;lt;b&amp;gt; &lt;c&gt; &quot;d&quot;",
    ))))
    .unwrap();
    assert_eq!(out.content.trim(), r#"a &lt;b&gt; <c> "d""#);
}

#[test]
fn non_ascii_text_survives_and_offsets_stay_on_char_boundaries() {
    let text = "caf\u{e9} \u{65e5}\u{672c} \u{1f600} end";
    let bytes = docx(&(para(text) + &para("second")));
    let out =
        extract_docx_v0_reader(Cursor::new(bytes), "mem.docx", ExtractLimits::default()).unwrap();
    let doc_text = &out.document.text;
    assert!(doc_text.contains(text), "got {doc_text:?}");

    let value = serde_json::to_value(&out).unwrap();
    for block in value["document"]["blocks"].as_array().unwrap() {
        let range = &block["doc_text_range"];
        let (start, end) = (
            range["start_byte"].as_u64().unwrap() as usize,
            range["end_byte"].as_u64().unwrap() as usize,
        );
        let slice = doc_text
            .get(start..end)
            .unwrap_or_else(|| panic!("range {start}..{end} not on char boundaries"));
        assert_eq!(slice, block["text"].as_str().unwrap());
    }
}

#[test]
fn text_split_across_runs_is_joined_in_order() {
    let body = r#"<w:p><w:r><w:t>Hel</w:t></w:r><w:r><w:t>lo, </w:t></w:r><w:r><w:t>world</w:t></w:r></w:p>"#;
    let out = extract_docx_reader(Cursor::new(docx(body))).unwrap();
    assert_eq!(out.content.trim(), "Hello, world");
}

#[test]
fn understated_entry_size_does_not_truncate_text() {
    // The declared uncompressed size is advisory; a header that under-reports
    // must not silently drop content.
    let long = "x".repeat(50_000);
    let mut bytes = docx(&para(&long));
    let mut patched = 0;
    for i in 0..bytes.len() - 4 {
        let off = match bytes[i..i + 4] {
            [0x50, 0x4b, 0x03, 0x04] => 22,
            [0x50, 0x4b, 0x01, 0x02] => 24,
            _ => continue,
        };
        bytes[i + off..i + off + 4].copy_from_slice(&64u32.to_le_bytes());
        patched += 1;
    }
    assert_eq!(patched, 2);
    let out = extract_docx_reader(Cursor::new(bytes)).unwrap();
    assert_eq!(out.content.trim(), long);
}

// --- numeric character references ------------------------------------------

fn pptx(slide_text: &str) -> Vec<u8> {
    const P_NS: &str = r#"xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main""#;
    let pres = format!(
        r#"{DECL}<p:presentation {P_NS} {R_NS}><p:sldIdLst><p:sldId id="256" r:id="rId1"/></p:sldIdLst></p:presentation>"#
    );
    let rels = format!(
        r#"{DECL}<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/></Relationships>"#
    );
    let slide = format!(
        r#"{DECL}<p:sld {P_NS}><p:cSld><p:spTree><p:sp><p:txBody><a:p><a:r><a:t>{slide_text}</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>"#
    );
    zip_of(&[
        ("ppt/presentation.xml", &pres),
        ("ppt/_rels/presentation.xml.rels", &rels),
        ("ppt/slides/slide1.xml", &slide),
    ])
}

const CHAR_REFS: &str = "caf&#233; &#x65E5;&#X672C; &#x1F600; &#38;lt;";
const CHAR_REFS_DECODED: &str = "caf\u{e9} \u{65e5}\u{672c} \u{1f600} &lt;";

#[test]
fn numeric_character_references_resolve_in_every_format() {
    let docx_out = extract_docx_reader(Cursor::new(docx(&para(CHAR_REFS)))).unwrap();
    assert_eq!(docx_out.content.trim(), CHAR_REFS_DECODED);

    let row = r#"<row r="1"><c r="A1" t="s"><v>0</v></c></row>"#;
    let xlsx_out = extract_xlsx_reader(Cursor::new(xlsx(row, &[CHAR_REFS]))).unwrap();
    assert_eq!(xlsx_out.content.trim(), CHAR_REFS_DECODED);

    let inline =
        format!(r#"<row r="1"><c r="A1" t="inlineStr"><is><t>{CHAR_REFS}</t></is></c></row>"#);
    let inline_out = extract_xlsx_reader(Cursor::new(xlsx(&inline, &[]))).unwrap();
    assert_eq!(inline_out.content.trim(), CHAR_REFS_DECODED);

    let pptx_out = extract_pptx_reader(Cursor::new(pptx(CHAR_REFS))).unwrap();
    assert_eq!(pptx_out.content.trim(), CHAR_REFS_DECODED);
}

#[test]
fn unresolvable_character_references_are_dropped() {
    // Policy (shared with the text/xml extractor): a reference that does not
    // name a Unicode scalar value is dropped; surrounding text is kept.
    let refs = "a&#xD800;b&#x110000;c&#99999999999;d&#xZZ;e&#;f&unknown;g";
    let out = extract_docx_reader(Cursor::new(docx(&para(refs)))).unwrap();
    assert_eq!(out.content.trim(), "abcdefg");
}
