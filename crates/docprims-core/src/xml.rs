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

/// Remove every character that is not an XML 1.0 `Char`.
///
/// Applied to extracted text before block byte ranges are computed, so the
/// output of every extractor contains only XML 1.0 characters.
pub fn retain_xml_chars(text: String) -> String {
    if text.chars().all(is_xml_char) {
        return text;
    }
    text.chars().filter(|&c| is_xml_char(c)).collect()
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
    fn malformed_references_are_rejected() {
        for reference in ["", "#", "#x", "#xZZ", "#-1", "#99999999999", "amp", "x41"] {
            assert_eq!(resolve_char_ref(reference), None, "{reference:?}");
        }
    }
}
