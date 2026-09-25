//! XML character rules shared by the extractors that parse XML.

/// Whether `c` is an XML 1.0 `Char` (XML 1.0 §2.2): tab, line feed, carriage
/// return, and the ranges U+0020–U+D7FF, U+E000–U+FFFD and U+10000–U+10FFFF.
pub fn is_xml_char(c: char) -> bool {
    matches!(c,
        '\t' | '\n' | '\r'
        | '\u{20}'..='\u{D7FF}'
        | '\u{E000}'..='\u{FFFD}'
        | '\u{10000}'..='\u{10FFFF}')
}

/// Whether `c` may appear in extracted text: an XML 1.0 `Char` that is not
/// DEL or a C1 control (U+007F–U+009F). XML 1.0 allows those but discourages
/// them, and they act as terminal controls.
pub fn is_output_char(c: char) -> bool {
    is_xml_char(c) && !matches!(c, '\u{7F}'..='\u{9F}')
}

/// Remove every character for which [`is_output_char`] is false.
///
/// Applied to extracted text before block byte ranges are computed, so the
/// output of every extractor contains no C0 controls other than tab, line
/// feed and carriage return, no DEL, no C1 controls, and nothing outside the
/// XML 1.0 character set.
pub fn retain_output_chars(text: String) -> String {
    if text.chars().all(is_output_char) {
        return text;
    }
    text.chars().filter(|&c| is_output_char(c)).collect()
}

/// Resolve the body of a numeric character reference (`#233` or `#xE9`, i.e.
/// the text between `&` and `;`).
///
/// Returns `None` if the reference is malformed or does not name an XML 1.0
/// `Char`; callers drop such references.
pub fn resolve_char_ref(reference: &str) -> Option<char> {
    let s = reference.strip_prefix('#')?;
    let code = match s.strip_prefix('x').or_else(|| s.strip_prefix('X')) {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => s.parse::<u32>().ok()?,
    };
    char::from_u32(code).filter(|&c| is_xml_char(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_decimal_and_hex_references_to_xml_chars() {
        assert_eq!(resolve_char_ref("#233"), Some('\u{e9}'));
        assert_eq!(resolve_char_ref("#xE9"), Some('\u{e9}'));
        assert_eq!(resolve_char_ref("#XE9"), Some('\u{e9}'));
        assert_eq!(resolve_char_ref("#x1F600"), Some('\u{1f600}'));
        assert_eq!(resolve_char_ref("#9"), Some('\t'));
        assert_eq!(resolve_char_ref("#xA"), Some('\n'));
        assert_eq!(resolve_char_ref("#xD"), Some('\r'));
    }

    #[test]
    fn boundaries_of_the_xml_char_ranges() {
        for (reference, allowed) in [
            ("#x1F", false),
            ("#x20", true),
            ("#xD7FF", true),
            ("#xD800", false),
            ("#xDFFF", false),
            ("#xE000", true),
            ("#xFFFD", true),
            ("#xFFFE", false),
            ("#xFFFF", false),
            ("#x10000", true),
            ("#x10FFFF", true),
            ("#x110000", false),
        ] {
            assert_eq!(
                resolve_char_ref(reference).is_some(),
                allowed,
                "{reference}"
            );
        }
    }

    #[test]
    fn control_characters_other_than_tab_lf_cr_are_rejected() {
        for code in (0u32..0x20).filter(|c| ![0x9, 0xA, 0xD].contains(c)) {
            assert_eq!(resolve_char_ref(&format!("#{code}")), None, "#{code}");
        }
    }

    #[test]
    fn output_chars_exclude_del_and_c1_but_keep_neighbours() {
        for c in ['\u{7F}', '\u{80}', '\u{85}', '\u{9B}', '\u{9F}'] {
            assert!(is_xml_char(c), "{c:?} is an XML Char");
            assert!(!is_output_char(c), "{c:?}");
        }
        for c in ['\u{7E}', '\u{A0}', '\t', '\n', '\r', 'é'] {
            assert!(is_output_char(c), "{c:?}");
        }
        assert_eq!(
            retain_output_chars("a\u{7f}b\u{9b}31mc\u{1b}d".into()),
            "ab31mcd"
        );
    }

    #[test]
    fn legacy_extracted_text_is_filtered() {
        assert_eq!(
            crate::ExtractedText::complete("a\u{1}b\u{9b}c".into()).content,
            "abc"
        );
        let partial = crate::ExtractedText::partial("a\u{1b}b".into(), "limit".into());
        assert_eq!(partial.content, "ab");
    }

    #[test]
    fn malformed_references_are_rejected() {
        for reference in ["", "#", "#x", "#xZZ", "#-1", "#99999999999", "amp", "x41"] {
            assert_eq!(resolve_char_ref(reference), None, "{reference:?}");
        }
    }
}
