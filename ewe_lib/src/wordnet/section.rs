//! Division of entries into `entries-<section>.yaml` files.
//!
//! The section of a lemma is derived from its first letter: diacritics are removed
//! (`école` -> `e`), letters of any script are lower-cased and used as is
//! (`σοφία` -> `σ`), Arabic `al-` and Hangul syllables are reduced to the first
//! consonant, all Han/kana characters share one `cjk` section and everything
//! else (digits, punctuation) goes to `0`.
use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

/// Internal key for the `entries-cjk.yaml` section (any CJK character is never
/// otherwise produced as a key).
pub(crate) const CJK_KEY: char = '字';

fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x3040..=0x30FF | 0x31F0..=0x31FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF
        | 0xF900..=0xFAFF | 0xFF66..=0xFF9F | 0x20000..=0x3FFFF)
}

/// The section key for a lemma
pub(crate) fn entry_key(lemma: &str) -> char {
    // Arabic definite article
    let lemma = match lemma.strip_prefix("ال") {
        Some(rest) if !rest.is_empty() => rest,
        _ => lemma,
    };
    let first = match lemma.nfd().find(|c| !is_combining_mark(*c)) {
        Some(c) => c,
        None => return '0',
    };
    if is_cjk(first) {
        return CJK_KEY;
    }
    if !first.is_alphabetic() {
        return '0';
    }
    match first.to_lowercase().next() {
        Some('ς') => 'σ',
        Some(c) => c,
        None => '0',
    }
}

/// The file name stem (`entries-<name>.yaml`) for a section key
pub(crate) fn key_to_name(key: char) -> String {
    if key == CJK_KEY {
        "cjk".to_string()
    } else {
        key.to_string()
    }
}

/// Inverse of `key_to_name`
pub(crate) fn name_to_key(name: &str) -> Option<char> {
    if name == "cjk" {
        return Some(CJK_KEY);
    }
    let mut cs = name.chars();
    match (cs.next(), cs.next()) {
        (Some(c), None) => Some(c),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keys() {
        assert_eq!(entry_key("apple"), 'a');
        assert_eq!(entry_key("Zebra"), 'z');
        assert_eq!(entry_key("école"), 'e');
        assert_eq!(entry_key("École"), 'e');
        assert_eq!(entry_key("e\u{301}cole"), 'e');
        assert_eq!(entry_key("Ärger"), 'a');
        assert_eq!(entry_key("ñu"), 'n');
        assert_eq!(entry_key("Éan"), 'e');
        assert_eq!(entry_key("άνθρωπος"), 'α');
        assert_eq!(entry_key("ςα"), 'σ');
        assert_eq!(entry_key("Москва"), 'м');
        assert_eq!(entry_key("йод"), 'и');
        assert_eq!(entry_key("한국"), '\u{1112}');
        assert_eq!(entry_key("漢字"), CJK_KEY);
        assert_eq!(entry_key("ひらがな"), CJK_KEY);
        assert_eq!(entry_key("カタカナ"), CJK_KEY);
        assert_eq!(entry_key("الكتاب"), 'ك');
        assert_eq!(entry_key("\u{64E}كتاب"), 'ك');
        assert_eq!(entry_key("שָׁלוֹם"), 'ש');
        assert_eq!(entry_key("किताब"), 'क');
        assert_eq!(entry_key("สวัสดี"), 'ส');
        assert_eq!(entry_key("አማርኛ"), 'አ');
        assert_eq!(entry_key("123"), '0');
        assert_eq!(entry_key("-x"), '0');
        assert_eq!(entry_key(""), '0');
    }

    #[test]
    fn test_names() {
        assert_eq!(key_to_name(CJK_KEY), "cjk");
        assert_eq!(name_to_key("cjk"), Some(CJK_KEY));
        assert_eq!(name_to_key("σ"), Some('σ'));
        assert_eq!(name_to_key("ab"), None);
        assert_eq!(name_to_key(&key_to_name('e')), Some('e'));
    }
}
