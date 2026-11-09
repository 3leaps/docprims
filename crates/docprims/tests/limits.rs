//! Output limits never produce empty blocks, stray separators or text beyond
//! the limit, at any limit value.

use std::io::{Cursor, Write};

use docprims::{DocprimsExtract, DocprimsQualityStatus, ExtractLimits};

fn zip_parts(parts: &[(&str, &str)]) -> Vec<u8> {
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, body) in parts {
        w.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        w.write_all(body.as_bytes()).unwrap();
    }
    w.finish().unwrap().into_inner()
}

const W_NS: &str = r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main""#;
const S_NS: &str = r#"xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main""#;
const R_NS: &str =
    r#"xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships""#;
const P_NS: &str = r#"xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main""#;

/// Block texts start with multi-byte characters so that truncating to a byte
/// limit can leave nothing of a block.
const BLOCKS: [&str; 3] = ["日本 one", "é two", "three"];

/// (source uri, input bytes, separator between blocks) for every format.
fn inputs() -> Vec<(&'static str, Vec<u8>, char)> {
    let [a, b, c] = BLOCKS;
    let docx = zip_parts(&[(
        "word/document.xml",
        &format!(
            "<w:document {W_NS}><w:body>{}</w:body></w:document>",
            BLOCKS
                .iter()
                .map(|t| format!("<w:p><w:r><w:t>{t}</w:t></w:r></w:p>"))
                .collect::<String>()
        ),
    )]);
    let rows: String = BLOCKS
        .iter()
        .enumerate()
        .map(|(i, t)| {
            format!(
                r#"<row r="{}"><c t="inlineStr"><is><t>{t}</t></is></c></row>"#,
                i + 1
            )
        })
        .collect();
    let xlsx = zip_parts(&[
        (
            "xl/workbook.xml",
            &format!(
                r#"<workbook {S_NS} {R_NS}><sheets><sheet name="S" sheetId="1" r:id="rId1"/></sheets></workbook>"#
            ),
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            &format!("<worksheet {S_NS}><sheetData>{rows}</sheetData></worksheet>"),
        ),
    ]);
    let paras: String = BLOCKS
        .iter()
        .map(|t| format!("<a:p><a:r><a:t>{t}</a:t></a:r></a:p>"))
        .collect();
    let pptx = zip_parts(&[
        (
            "ppt/presentation.xml",
            &format!(r#"<p:presentation {P_NS} {R_NS}><p:sldIdLst><p:sldId id="256" r:id="rId1"/></p:sldIdLst></p:presentation>"#),
        ),
        (
            "ppt/_rels/presentation.xml.rels",
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/></Relationships>"#,
        ),
        (
            "ppt/slides/slide1.xml",
            &format!("<p:sld {P_NS}><p:cSld><p:spTree><p:sp><p:txBody>{paras}</p:txBody></p:sp></p:spTree></p:cSld></p:sld>"),
        ),
    ]);
    vec![
        ("m.md", format!("{a}\n\n{b}\n\n{c}\n").into_bytes(), '\n'),
        (
            "h.html",
            format!("<p>{a}</p><p>{b}</p><p>{c}</p>").into_bytes(),
            '\n',
        ),
        (
            "x.xml",
            format!("<r><a>{a}</a><b>{b}</b><c>{c}</c></r>").into_bytes(),
            ' ',
        ),
        ("d.docx", docx, '\n'),
        ("s.xlsx", xlsx, '\n'),
        ("p.pptx", pptx, '\n'),
    ]
}

fn check(label: &str, out: &DocprimsExtract, limit: usize, full_len: usize, sep: char) {
    let doc = &out.document;
    assert!(
        doc.text.len() <= limit,
        "{label}: {} bytes > limit",
        doc.text.len()
    );

    // The text is exactly the blocks joined by single separators.
    let mut rebuilt = String::new();
    for (i, block) in doc.blocks.iter().enumerate() {
        assert!(!block.text.is_empty(), "{label}: empty block {}", block.id);
        if i > 0 {
            rebuilt.push(sep);
        }
        let r = &block.doc_text_range;
        assert_eq!(
            r.start_byte,
            rebuilt.len(),
            "{label}: block {} start",
            block.id
        );
        rebuilt.push_str(&block.text);
        assert_eq!(r.end_byte, rebuilt.len(), "{label}: block {} end", block.id);
    }
    assert_eq!(doc.text, rebuilt, "{label}: text is not the joined blocks");

    let cut = doc.text.len() < full_len;
    assert_eq!(
        doc.quality.status == DocprimsQualityStatus::Partial,
        cut,
        "{label}: quality {:?} with {} of {full_len} bytes",
        doc.quality,
        doc.text.len()
    );
}

#[test]
fn every_output_limit_yields_well_formed_output() {
    for (uri, bytes, sep) in inputs() {
        let full = docprims::extract_bytes(uri, &bytes, ExtractLimits::default()).unwrap();
        let full_len = full.document.text.len();
        assert_eq!(full.document.blocks.len(), 3, "{uri}: fixture blocks");
        for limit in 0..=full_len + 2 {
            let limits = ExtractLimits {
                max_output_bytes: limit,
                ..ExtractLimits::default()
            };
            let out = docprims::extract_bytes(uri, &bytes, limits).unwrap();
            check(&format!("{uri} limit={limit}"), &out, limit, full_len, sep);
        }
    }
}

#[test]
fn zero_output_limit_yields_no_blocks() {
    let limits = ExtractLimits {
        max_output_bytes: 0,
        ..ExtractLimits::default()
    };
    for (uri, bytes, _) in inputs() {
        let out = docprims::extract_bytes(uri, &bytes, limits).unwrap();
        assert!(out.document.blocks.is_empty(), "{uri}");
        assert_eq!(out.document.text, "", "{uri}");
        assert_eq!(
            out.document.quality.status,
            DocprimsQualityStatus::Partial,
            "{uri}"
        );
    }
}
