use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use docprims_core::{DocprimsExtract, ExtractLimits};
use jsonschema::Resource;
use std::io::Cursor;
use std::path::{Path, PathBuf};

fn limits() -> ExtractLimits {
    ExtractLimits {
        max_input_bytes: 1024 * 1024,
        max_output_bytes: 1024 * 1024,
        max_blocks: 10_000,
    }
}

fn v0_validator() -> jsonschema::Validator {
    let root_schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schemas/v0/extract/docprims-extract.schema.json"
    ))
    .unwrap();

    let block_schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schemas/v0/extract/docprims-block.schema.json"
    ))
    .unwrap();

    let loc_schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schemas/v0/extract/docprims-location.schema.json"
    ))
    .unwrap();

    jsonschema::draft202012::options()
        .with_resources(
            [
                (
                    "https://schemas.3leaps.dev/docprims/extract/v0/docprims-block.schema.json",
                    Resource::from_contents(block_schema).unwrap(),
                ),
                (
                    "https://schemas.3leaps.dev/docprims/extract/v0/docprims-location.schema.json",
                    Resource::from_contents(loc_schema).unwrap(),
                ),
            ]
            .into_iter(),
        )
        .build(&root_schema)
        .unwrap()
}

fn read_text_fixture(path: &str) -> String {
    std::fs::read_to_string(repo_root().join(path)).unwrap()
}

fn read_b64_fixture(path: &str) -> Vec<u8> {
    let s = std::fs::read_to_string(repo_root().join(path)).unwrap();
    let compact: String = s.lines().map(|l| l.trim()).collect();
    STANDARD.decode(compact.as_bytes()).unwrap()
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points at crates/docprims-cli; repo root is two levels up.
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn assert_golden(name: &str, got: &DocprimsExtract) {
    let golden_path = repo_root().join(format!("testdata/golden/v0/{name}.json"));
    let expected: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(golden_path).unwrap()).unwrap();
    let got_value = serde_json::to_value(got).unwrap();

    let validator = v0_validator();
    let errors: Vec<_> = validator.iter_errors(&got_value).collect();
    assert!(errors.is_empty(), "schema errors: {errors:?}");

    assert_eq!(got_value, expected);
}

#[test]
fn golden_markdown_simple_v0() {
    let src = "testdata/fixtures/text/simple.md";
    let s = read_text_fixture(src);
    let got = docprims_text::markdown::extract_v0_str(&s, src, limits()).unwrap();
    assert_golden("markdown-simple", &got);
}

#[test]
fn golden_html_simple_v0() {
    let src = "testdata/fixtures/text/simple.html";
    let s = read_text_fixture(src);
    let got = docprims_text::html::extract_v0_str(&s, src, limits()).unwrap();
    assert_golden("html-simple", &got);
}

#[test]
fn golden_xml_simple_v0() {
    let src = "testdata/fixtures/text/simple.xml";
    let s = read_text_fixture(src);
    let got = docprims_text::xml::extract_v0_str(&s, src, limits()).unwrap();
    assert_golden("xml-simple", &got);
}

#[test]
fn golden_docx_mini_v0() {
    let src = "testdata/fixtures/ooxml/mini.docx";
    let zip_bytes = read_b64_fixture("testdata/fixtures/ooxml/mini.docx.b64");
    let got =
        docprims_ooxml::extract_docx_v0_reader(Cursor::new(zip_bytes), src, limits()).unwrap();
    assert_golden("docx-mini", &got);
}

#[test]
fn golden_xlsx_mini_v0() {
    let src = "testdata/fixtures/ooxml/mini.xlsx";
    let zip_bytes = read_b64_fixture("testdata/fixtures/ooxml/mini.xlsx.b64");
    let got =
        docprims_ooxml::extract_xlsx_v0_reader(Cursor::new(zip_bytes), src, limits()).unwrap();
    assert_golden("xlsx-mini", &got);
}

#[test]
fn golden_pptx_mini_v0() {
    let src = "testdata/fixtures/ooxml/mini.pptx";
    let zip_bytes = read_b64_fixture("testdata/fixtures/ooxml/mini.pptx.b64");
    let got =
        docprims_ooxml::extract_pptx_v0_reader(Cursor::new(zip_bytes), src, limits()).unwrap();
    assert_golden("pptx-mini", &got);
}
