//! Common utilities for OOXML parsing.

use docprims_core::{DocprimsError, Result};
use quick_xml::escape::resolve_xml_entity;
use std::borrow::Cow;
use std::io::{Read, Seek};
use zip::ZipArchive;

use encoding_rs::Encoding;

/// Maximum decompressed size of any single archive part (100 MB default).
pub const MAX_DECOMPRESSED_SIZE: u64 = 100 * 1024 * 1024;

/// Maximum decompressed size summed over every part read from one archive.
pub const MAX_TOTAL_DECOMPRESSED_SIZE: u64 = 4 * MAX_DECOMPRESSED_SIZE;

/// Maximum number of files in archive to prevent DoS.
pub const MAX_ARCHIVE_FILES: usize = 10_000;

/// Upper bound on buffer pre-allocation. The header-declared size is untrusted,
/// so it only sizes the initial buffer up to this cap.
const MAX_PREALLOC: u64 = 1024 * 1024;

/// A ZIP archive opened for OOXML extraction.
///
/// Carries a decompression budget shared by every part read from it, so an
/// archive of many individually legal parts cannot inflate without bound.
pub struct OoxmlArchive<R> {
    zip: ZipArchive<R>,
    part_limit: u64,
    remaining: u64,
}

/// Open a ZIP archive with safety limits.
pub fn open_archive<R: Read + Seek>(reader: R) -> Result<OoxmlArchive<R>> {
    open_archive_with_limits(reader, MAX_DECOMPRESSED_SIZE, MAX_TOTAL_DECOMPRESSED_SIZE)
}

fn open_archive_with_limits<R: Read + Seek>(
    reader: R,
    part_limit: u64,
    total_limit: u64,
) -> Result<OoxmlArchive<R>> {
    let zip = ZipArchive::new(reader)
        .map_err(|e| DocprimsError::Malformed(format!("Invalid ZIP archive: {}", e)))?;

    // Check file count limit
    if zip.len() > MAX_ARCHIVE_FILES {
        return Err(DocprimsError::ResourceLimit(format!(
            "Archive contains too many files: {} (max {})",
            zip.len(),
            MAX_ARCHIVE_FILES
        )));
    }

    Ok(OoxmlArchive {
        zip,
        part_limit,
        remaining: total_limit,
    })
}

/// Read a file from the archive, bounding the bytes actually decompressed.
///
/// The declared uncompressed size in the archive header is attacker-controlled
/// and is used only as an early reject; the read itself is capped at the
/// smaller of the per-part limit and the archive's remaining budget.
pub fn read_archive_file<R: Read + Seek>(
    archive: &mut OoxmlArchive<R>,
    name: &str,
) -> Result<Option<String>> {
    let part_too_large = || {
        DocprimsError::ResourceLimit(format!(
            "File {} too large: decompressed size exceeds {} bytes",
            name, archive.part_limit
        ))
    };
    let total_too_large = || {
        DocprimsError::ResourceLimit(format!(
            "Archive too large: total decompressed size exceeds budget at {}",
            name
        ))
    };

    let file = match archive.zip.by_name(name) {
        Ok(f) => f,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => {
            return Err(DocprimsError::Malformed(format!(
                "Error reading {}: {}",
                name, e
            )))
        }
    };

    let declared = file.size();
    if declared > archive.part_limit {
        return Err(part_too_large());
    }
    if declared > archive.remaining {
        return Err(total_too_large());
    }

    let limit = archive.part_limit.min(archive.remaining);
    let Some(content) = read_bounded(file, name, limit, declared)? else {
        return Err(if limit < archive.part_limit {
            total_too_large()
        } else {
            part_too_large()
        });
    };
    archive.remaining -= content.len() as u64;

    Ok(Some(decode_xml_bytes(&content)?))
}

/// Read at most `limit` bytes. Returns `None`, without reading further, if the
/// stream holds more. `size_hint` sizes the initial buffer, capped at
/// `MAX_PREALLOC`.
fn read_bounded<R: Read>(
    reader: R,
    name: &str,
    limit: u64,
    size_hint: u64,
) -> Result<Option<Vec<u8>>> {
    let mut content = Vec::with_capacity(size_hint.min(limit).min(MAX_PREALLOC) as usize);
    reader
        .take(limit.saturating_add(1))
        .read_to_end(&mut content)
        .map_err(|e| DocprimsError::Malformed(format!("Error reading {}: {}", name, e)))?;
    if content.len() as u64 > limit {
        return Ok(None);
    }
    Ok(Some(content))
}

fn decode_xml_bytes(bytes: &[u8]) -> Result<String> {
    let (enc, bom_len) = detect_encoding_and_bom(bytes);
    let bytes = bytes.get(bom_len..).unwrap_or(bytes);
    let (decoded, _, had_errors) = enc.decode(bytes);
    // Best-effort decoding: allow replacement chars.
    // Once v0 structured outputs are emitted for OOXML, propagate a warning when had_errors is true.
    let _ = had_errors;
    Ok(match decoded {
        Cow::Borrowed(s) => s.to_string(),
        Cow::Owned(s) => s,
    })
}

fn detect_encoding_and_bom(bytes: &[u8]) -> (&'static Encoding, usize) {
    // BOM detection first.
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return (encoding_rs::UTF_8, 3);
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return (encoding_rs::UTF_16LE, 2);
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        return (encoding_rs::UTF_16BE, 2);
    }

    // Best-effort XML declaration sniffing.
    // NOTE: In OOXML we primarily expect UTF-8 or UTF-16 with a BOM.
    // To avoid mis-decoding on attacker-controlled `encoding=...` declarations,
    // only honor UTF-8 declarations when no BOM is present.
    if let Some(label) = sniff_xml_decl_encoding(bytes) {
        let norm = label.trim().to_ascii_lowercase();
        if norm == "utf-8" || norm == "utf8" {
            return (encoding_rs::UTF_8, 0);
        }
    }

    (encoding_rs::UTF_8, 0)
}

fn sniff_xml_decl_encoding(bytes: &[u8]) -> Option<String> {
    // XML declaration is ASCII-compatible; only scan a small prefix.
    let prefix_len = bytes.len().min(1024);
    let prefix = &bytes[..prefix_len];
    let s = std::str::from_utf8(prefix).ok()?;
    let lower = s.to_ascii_lowercase();
    if !lower.starts_with("<?xml") {
        return None;
    }

    let idx = lower.find("encoding")?;
    let mut rest = &s[idx + "encoding".len()..];

    rest = rest
        .strip_prefix(|c: char| c.is_ascii_whitespace())
        .unwrap_or(rest);
    while let Some(r) = rest.strip_prefix(|c: char| c.is_ascii_whitespace()) {
        rest = r;
    }

    rest = rest.strip_prefix('=')?;
    while let Some(r) = rest.strip_prefix(|c: char| c.is_ascii_whitespace()) {
        rest = r;
    }

    let mut chars = rest.chars();
    let quote = chars.next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }

    let after_quote = chars.as_str();
    let end = after_quote.find(quote)?;
    Some(after_quote[..end].trim().to_string())
}

/// Resolve an XML entity reference to its string value.
///
/// Handles predefined XML entities (lt, gt, amp, apos, quot) and numeric
/// character references (&#NNN; or &#xHHH;). A numeric reference resolves
/// only when it names an XML 1.0 `Char` (see `docprims_core::xml`); other
/// numeric references and unknown named entities resolve to `None`.
pub fn resolve_entity(name: &str) -> Option<Cow<'static, str>> {
    if let Some(resolved) = resolve_xml_entity(name) {
        return Some(Cow::Borrowed(resolved));
    }
    docprims_core::xml::resolve_char_ref(name).map(|c| Cow::Owned(c.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    use zip::write::SimpleFileOptions;

    /// Build a single-entry deflated archive. If `declared_size` is set, the
    /// uncompressed-size fields in both the local and central headers are
    /// rewritten to that value, as a hostile archive would.
    fn archive_with(name: &str, content: &[u8], declared_size: Option<u32>) -> Vec<u8> {
        let mut bytes = archive_of(&[(name, content)]);
        if let Some(size) = declared_size {
            patch_uncompressed_size(&mut bytes, size);
        }
        bytes
    }

    fn archive_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let opts =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, content) in entries {
            w.start_file(*name, opts).unwrap();
            w.write_all(content).unwrap();
        }
        w.finish().unwrap().into_inner()
    }

    fn patch_uncompressed_size(bytes: &mut [u8], size: u32) {
        let mut patched = 0;
        for i in 0..bytes.len().saturating_sub(4) {
            let offset = match bytes[i..i + 4] {
                [0x50, 0x4b, 0x03, 0x04] => 22, // local file header
                [0x50, 0x4b, 0x01, 0x02] => 24, // central directory header
                _ => continue,
            };
            bytes[i + offset..i + offset + 4].copy_from_slice(&size.to_le_bytes());
            patched += 1;
        }
        assert_eq!(patched, 2, "expected one local and one central header");
    }

    fn read(bytes: Vec<u8>, limit: u64) -> Result<Option<String>> {
        let mut archive = open_archive_with_limits(Cursor::new(bytes), limit, u64::MAX).unwrap();
        read_archive_file(&mut archive, "part.xml")
    }

    #[test]
    fn lying_header_cannot_bypass_decompression_limit() {
        let content = vec![b'a'; 64 * 1024];
        let bytes = archive_with("part.xml", &content, Some(100));
        match read(bytes, 16 * 1024) {
            Err(DocprimsError::ResourceLimit(msg)) => assert!(msg.contains("exceeds 16384")),
            other => panic!("expected ResourceLimit, got {other:?}"),
        }
    }

    #[test]
    fn decompression_limit_is_inclusive() {
        let limit = 4096;
        let at_limit = archive_with("part.xml", &vec![b'a'; limit], None);
        assert_eq!(read(at_limit, limit as u64).unwrap().unwrap().len(), limit);

        let over = archive_with("part.xml", &vec![b'a'; limit + 1], None);
        assert!(matches!(
            read(over, limit as u64),
            Err(DocprimsError::ResourceLimit(_))
        ));
    }

    #[test]
    fn declared_size_over_limit_is_rejected_before_reading() {
        let bytes = archive_with("part.xml", b"<x/>", Some(u32::MAX));
        match read(bytes, 1024) {
            // Content is 4 bytes, so only the declared size can trigger this.
            Err(DocprimsError::ResourceLimit(_)) => {}
            other => panic!("expected ResourceLimit, got {other:?}"),
        }
    }

    #[test]
    fn understated_size_within_limit_reads_full_content() {
        // A header that under-reports must not truncate what we extract.
        let content = "<x>".to_string() + &"y".repeat(10_000) + "</x>";
        let bytes = archive_with("part.xml", content.as_bytes(), Some(10));
        assert_eq!(read(bytes, 1024 * 1024).unwrap().unwrap(), content);
    }

    /// Endless source that fails the test if pulled past `allowed` bytes.
    struct Tripwire {
        pulled: u64,
        allowed: u64,
    }

    impl Read for Tripwire {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            assert!(
                self.pulled + buf.len() as u64 <= self.allowed,
                "read past the limit: {} bytes already pulled",
                self.pulled
            );
            buf.fill(b'a');
            self.pulled += buf.len() as u64;
            Ok(buf.len())
        }
    }

    #[test]
    fn bounded_read_stops_at_limit_on_endless_stream() {
        let limit = 8192;
        let src = Tripwire {
            pulled: 0,
            allowed: limit + 1,
        };
        assert!(read_bounded(src, "part.xml", limit, 10).unwrap().is_none());
    }

    #[test]
    fn bounded_read_ignores_huge_size_hint_for_allocation() {
        let content = read_bounded(&b"<x/>"[..], "part.xml", MAX_DECOMPRESSED_SIZE, u64::MAX)
            .unwrap()
            .unwrap();
        assert_eq!(content, b"<x/>");
        assert!(content.capacity() <= MAX_PREALLOC as usize);
    }

    #[test]
    fn missing_part_is_none_not_error() {
        let bytes = archive_with("other.xml", b"<x/>", None);
        assert!(read(bytes, 1024).unwrap().is_none());
    }

    fn three_parts(each: usize) -> Vec<u8> {
        let body = vec![b'a'; each];
        archive_of(&[("p1.xml", &body), ("p2.xml", &body), ("p3.xml", &body)])
    }

    #[test]
    fn aggregate_budget_spans_parts_that_are_individually_legal() {
        // Each part is under the per-part limit; together they exceed the total.
        let mut archive =
            open_archive_with_limits(Cursor::new(three_parts(3000)), 4096, 8192).unwrap();
        assert!(read_archive_file(&mut archive, "p1.xml").unwrap().is_some());
        assert!(read_archive_file(&mut archive, "p2.xml").unwrap().is_some());
        match read_archive_file(&mut archive, "p3.xml") {
            Err(DocprimsError::ResourceLimit(msg)) => assert!(msg.contains("total"), "{msg}"),
            other => panic!("expected aggregate ResourceLimit, got {other:?}"),
        }
    }

    #[test]
    fn aggregate_budget_holds_when_headers_understate_sizes() {
        let mut bytes = three_parts(3000);
        let mut patched = 0;
        for i in 0..bytes.len() - 4 {
            let offset = match bytes[i..i + 4] {
                [0x50, 0x4b, 0x03, 0x04] => 22,
                [0x50, 0x4b, 0x01, 0x02] => 24,
                _ => continue,
            };
            bytes[i + offset..i + offset + 4].copy_from_slice(&1u32.to_le_bytes());
            patched += 1;
        }
        assert_eq!(patched, 6);
        let mut archive = open_archive_with_limits(Cursor::new(bytes), 4096, 8192).unwrap();
        read_archive_file(&mut archive, "p1.xml").unwrap();
        read_archive_file(&mut archive, "p2.xml").unwrap();
        assert!(matches!(
            read_archive_file(&mut archive, "p3.xml"),
            Err(DocprimsError::ResourceLimit(_))
        ));
    }

    #[test]
    fn declared_size_over_remaining_budget_is_rejected_before_reading() {
        // Content is 4 bytes, so only the declared size can trigger this.
        let bytes = archive_with("part.xml", b"<x/>", Some(5000));
        let mut archive = open_archive_with_limits(Cursor::new(bytes), 8192, 4096).unwrap();
        match read_archive_file(&mut archive, "part.xml") {
            Err(DocprimsError::ResourceLimit(msg)) => assert!(msg.contains("total"), "{msg}"),
            other => panic!("expected aggregate ResourceLimit, got {other:?}"),
        }
    }

    #[test]
    fn missing_parts_do_not_consume_budget() {
        let mut archive =
            open_archive_with_limits(Cursor::new(three_parts(3000)), 4096, 6000).unwrap();
        for _ in 0..10 {
            assert!(read_archive_file(&mut archive, "absent.xml")
                .unwrap()
                .is_none());
        }
        read_archive_file(&mut archive, "p1.xml").unwrap();
        read_archive_file(&mut archive, "p2.xml").unwrap();
    }

    #[test]
    fn sniff_xml_decl_encoding_handles_whitespace() {
        let s = b"<?xml version=\"1.0\" encoding = \"UTF-16\"?><x/>";
        assert_eq!(sniff_xml_decl_encoding(s).as_deref(), Some("UTF-16"));

        let s = b"<?xml version=\"1.0\"\n encoding=\"UTF-8\"?><x/>";
        assert_eq!(sniff_xml_decl_encoding(s).as_deref(), Some("UTF-8"));
    }

    #[test]
    fn detect_encoding_prefers_utf8_without_bom() {
        let s = b"<?xml version=\"1.0\" encoding=\"UTF-16\"?><x/>";
        let (enc, bom_len) = detect_encoding_and_bom(s);
        assert_eq!(enc.name(), "UTF-8");
        assert_eq!(bom_len, 0);

        let s = b"<?xml version=\"1.0\" encoding=\"utf-8\"?><x/>";
        let (enc, bom_len) = detect_encoding_and_bom(s);
        assert_eq!(enc.name(), "UTF-8");
        assert_eq!(bom_len, 0);
    }

    #[test]
    fn decode_xml_bytes_honors_utf16_bom() {
        // UTF-16LE BOM + "<a/>"
        let bytes = [0xFF, 0xFE, b'<', 0x00, b'a', 0x00, b'/', 0x00, b'>', 0x00];
        let out = decode_xml_bytes(&bytes).unwrap();
        assert_eq!(out, "<a/>");
    }
}
