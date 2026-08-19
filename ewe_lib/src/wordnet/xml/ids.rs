//! WN-LMF `@id`/`@target`/`@members` construction and parsing.
//!
//! EWE's internal `SenseId` is a real WordNet sense key (e.g. `foot%1:08:00::`) and lemmas can
//! contain arbitrary characters, but WN-LMF requires `Sense`/`LexicalEntry`/`Synset` `@id` to be
//! a legal XML `ID` (an NMTOKEN - no `%`, `:`, spaces, ...). This module ports OEWN's own
//! reversible escaping scheme for that translation
//! (`scripts/sense_keys.py::{escape,unescape,map,unmap}_sense_key` and
//! `scripts/wordnet.py::escape_lemma` in the `globalwordnet/english-wordnet` repo), so that
//! importing OEWN's real `wn.xml` recovers the exact original sense keys (rather than
//! re-deriving them, which risks drifting from the original if reconstruction order differs from
//! how they were first assigned) while staying a no-op - and therefore still correct - on ids
//! from other GWA projects that don't use this convention at all.

/// Escape the punctuation in a sense key's lemma portion into an XML `Name`-safe form. Order
/// matters: literal `-` is doubled to `--` *first*, so every other substitution below produces
/// single-hyphen-delimited tokens (`-apos-`, `-colon-`, ...) that can never be confused with a
/// real hyphen once escaped.
pub fn escape_sense_key(s: &str) -> String {
    s.replace('-', "--")
        .replace('\'', "-apos-")
        .replace('!', "-excl-")
        .replace('#', "-num-")
        .replace('$', "-dollar-")
        .replace('%', "-percnt-")
        .replace('&', "-amp-")
        .replace('(', "-lpar-")
        .replace(')', "-rpar-")
        .replace('*', "-ast-")
        .replace('+', "-plus-")
        .replace(',', "-comma-")
        .replace('/', "-sol-")
        .replace('{', "-lbrace-")
        .replace('|', "-vert-")
        .replace('}', "-rbrace-")
        .replace('~', "-tilde-")
        .replace('¢', "-cent-")
        .replace('£', "-pound-")
        .replace('§', "-sect-")
        .replace('©', "-copy-")
        .replace('®', "-reg-")
        .replace('°', "-deg-")
        .replace('´', "-acute-")
        .replace('¶', "-para-")
        .replace('º', "-ordm-")
        .replace(':', "-colon-")
}

/// Inverse of [`escape_sense_key`]. The specific-token replacements must all run before the final
/// `--` -> `-` collapse: two adjacent tokens (e.g. `'!` escapes to `-apos--excl-`) leave a `--`
/// in the escaped string that is *not* an escaped literal hyphen, and would be corrupted by
/// collapsing it too early.
pub fn unescape_sense_key(s: &str) -> String {
    s.replace("-apos-", "'")
        .replace("-colon-", ":")
        .replace("-excl-", "!")
        .replace("-num-", "#")
        .replace("-dollar-", "$")
        .replace("-percnt-", "%")
        .replace("-amp-", "&")
        .replace("-lpar-", "(")
        .replace("-rpar-", ")")
        .replace("-ast-", "*")
        .replace("-plus-", "+")
        .replace("-comma-", ",")
        .replace("-sol-", "/")
        .replace("-lbrace-", "{")
        .replace("-vert-", "|")
        .replace("-rbrace-", "}")
        .replace("-tilde-", "~")
        .replace("-cent-", "¢")
        .replace("-pound-", "£")
        .replace("-sect-", "§")
        .replace("-copy-", "©")
        .replace("-reg-", "®")
        .replace("-deg-", "°")
        .replace("-acute-", "´")
        .replace("-para-", "¶")
        .replace("-ordm-", "º")
        .replace("--", "-")
}

/// Maps a real sense key (`lemma%ss_type:lex_filenum:lex_id:head_word:head_id`) to a legal
/// WN-LMF `Sense/@id`. A sense key with more than one `%` means the lemma itself contains a
/// literal `%` (the split-on-last-`%` handles that, since the trailing info fields never contain
/// one).
pub fn map_sense_key(sense_key: &str, prefix: &str) -> String {
    match sense_key.rsplit_once('%') {
        Some((lemma, info)) => format!(
            "{}-{}__{}",
            prefix,
            escape_sense_key(lemma),
            info.replace('_', "-sp-").replace(':', ".")
        ),
        None => format!("{}-{}", prefix, escape_sense_key(sense_key)),
    }
}

/// Inverse of [`map_sense_key`]. Assumes `xml_id` starts with `{prefix}-` (true of any id this
/// module produced, and of any real OEWN sense id); a `prefix` that isn't actually a prefix of
/// `xml_id` leaves the id effectively unparsed (empty/garbage lemma), which import surfaces as a
/// malformed-document error rather than silently misparsing.
pub fn unmap_sense_key(xml_id: &str, prefix: &str) -> String {
    let key_prefix_len = prefix.len() + 1;
    let rest = xml_id.get(key_prefix_len..).unwrap_or("");
    match rest.split_once("__") {
        Some((oewn_key, info)) => format!(
            "{}%{}",
            unescape_sense_key(oewn_key),
            info.replace("-sp-", "_").replace('.', ":")
        ),
        None => unescape_sense_key(rest),
    }
}

/// Format a lemma into a valid XML `Name` for use inside a `LexicalEntry/@id`. Letters, digits,
/// `.`, and `-` pass through as-is (XML `Name` already permits broad Unicode letter ranges,
/// unlike NMTOKEN-safety for punctuation); everything else gets a short mnemonic escape, falling
/// back to a `-uXXXX-` hex escape for anything not explicitly listed.
pub fn escape_lemma(lemma: &str) -> String {
    let mut out = String::with_capacity(lemma.len());
    for c in lemma.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '.' | '-' => out.push(c),
            ' ' => out.push('_'),
            '(' => out.push_str("-lb-"),
            ')' => out.push_str("-rb-"),
            '\'' => out.push_str("-ap-"),
            '/' => out.push_str("-sl-"),
            ':' => out.push_str("-cn-"),
            ',' => out.push_str("-cm-"),
            '!' => out.push_str("-ex-"),
            '+' => out.push_str("-pl-"),
            '%' => out.push_str("-pc-"),
            _ if is_xml_name_char(c) => out.push(c),
            _ => out.push_str(&format!("-u{:04X}-", c as u32)),
        }
    }
    out
}

/// A conservative approximation of the XML 1.0 `NameChar` production - covers the common ranges
/// (Latin-1 Supplement letters, combining marks, extended Latin) `escape_lemma` needs to leave
/// unescaped; anything outside it falls back to the `-uXXXX-` hex escape, which is always safe.
fn is_xml_name_char(c: char) -> bool {
    c == '_'
        || c == '\u{B7}'
        || matches!(c, '\u{C0}'..='\u{D6}')
        || matches!(c, '\u{D8}'..='\u{F6}')
        || matches!(c, '\u{F8}'..='\u{2FF}')
        || matches!(c, '\u{370}'..='\u{37D}')
        || matches!(c, '\u{37F}'..='\u{1FFF}')
        || matches!(c, '\u{200C}'..='\u{200D}')
        || matches!(c, '\u{2070}'..='\u{218F}')
        || matches!(c, '\u{2C00}'..='\u{2FEF}')
        || matches!(c, '\u{3001}'..='\u{D7FF}')
        || matches!(c, '\u{F900}'..='\u{FDCF}')
        || matches!(c, '\u{FDF0}'..='\u{FFFD}')
        || matches!(c, '\u{300}'..='\u{36F}')
        || matches!(c, '\u{203F}'..='\u{2040}')
}

/// `LexicalEntry/@id`: `{prefix}-{escaped lemma}-{pos key}`. The pos key is the full `PosKey`
/// string (e.g. `n`, `n2` for a homograph entry), matching how OEWN's `from_yaml.py` builds this
/// id from its YAML `pos_map` keys.
pub fn entry_xml_id(prefix: &str, lemma: &str, poskey: &crate::wordnet::PosKey) -> String {
    format!("{}-{}-{}", prefix, escape_lemma(lemma), poskey.as_str())
}

/// `Synset/@id`: synset ids (`00001740-n`) are already legal `Name` characters, so this is just
/// prefixing - no escaping needed.
pub fn synset_xml_id(prefix: &str, id: &crate::wordnet::SynsetId) -> String {
    format!("{}-{}", prefix, id.as_str())
}

/// `Sense/@id`: the real sense key, mapped via [`map_sense_key`].
pub fn sense_xml_id(prefix: &str, sense_id: &crate::wordnet::SenseId) -> String {
    map_sense_key(sense_id.as_str(), prefix)
}

/// Inverse of [`synset_xml_id`] - strips the `{prefix}-` a plain synset-id-shaped XML id (a
/// `Synset/@id`/`@target`, or a `Sense/@synset`) was built with. Synset ids contain none of the
/// characters [`escape_sense_key`] would have touched, so - unlike [`unmap_sense_key`] - this is
/// a plain prefix strip, not an unescape.
pub fn strip_prefix_id(prefix: &str, xml_id: &str) -> String {
    xml_id
        .strip_prefix(prefix)
        .and_then(|s| s.strip_prefix('-'))
        .unwrap_or(xml_id)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_unescape_sense_key_round_trip() {
        for s in [
            "foot",
            "it's",
            "50/50",
            "a:b",
            "a-b",
            "a--b",
            "a'!b",
            "100%",
            ".22-caliber",
        ] {
            assert_eq!(unescape_sense_key(&escape_sense_key(s)), s, "round trip of {s:?}");
        }
    }

    #[test]
    fn test_escape_sense_key_adjacent_tokens() {
        // Two adjacent escape tokens leave a "--" that must NOT be treated as an escaped
        // literal hyphen when unescaping.
        assert_eq!(escape_sense_key("'!"), "-apos--excl-");
        assert_eq!(unescape_sense_key("-apos--excl-"), "'!");
    }

    #[test]
    fn test_map_sense_key_matches_real_oewn_output() {
        // Transcribed straight from english-wordnet-2025.xml.gz.
        assert_eq!(map_sense_key("foot%1:08:00::", "oewn"), "oewn-foot__1.08.00..");
        assert_eq!(map_sense_key(".22%1:06:00::", "oewn"), "oewn-.22__1.06.00..");
        assert_eq!(
            map_sense_key(".22-caliber%3:01:00::", "oewn"),
            "oewn-.22--caliber__3.01.00.."
        );
    }

    #[test]
    fn test_unmap_sense_key_matches_real_oewn_output() {
        assert_eq!(unmap_sense_key("oewn-foot__1.08.00..", "oewn"), "foot%1:08:00::");
        assert_eq!(unmap_sense_key("oewn-.22__1.06.00..", "oewn"), ".22%1:06:00::");
        assert_eq!(
            unmap_sense_key("oewn-.22--caliber__3.01.00..", "oewn"),
            ".22-caliber%3:01:00::"
        );
    }

    #[test]
    fn test_map_unmap_sense_key_round_trip() {
        for sk in [
            "foot%1:08:00::",
            "abate%2:30:01::",
            "scorching%5:00:01:hot:01",
            "it's%1:10:00::",
            "50%%1:23:00::",
        ] {
            assert_eq!(unmap_sense_key(&map_sense_key(sk, "oewn"), "oewn"), sk, "round trip of {sk:?}");
        }
    }

    #[test]
    fn test_map_sense_key_lemma_with_percent() {
        // A literal '%' inside the lemma must not be confused with the sense-key delimiter -
        // only the last '%' separates lemma from the info fields.
        assert_eq!(
            map_sense_key("50%%1:23:00::", "oewn"),
            "oewn-50-percnt-__1.23.00.."
        );
    }

    #[test]
    fn test_escape_lemma_matches_real_oewn_output() {
        assert_eq!(escape_lemma("'hood"), "-ap-hood");
        assert_eq!(escape_lemma(".22-caliber"), ".22-caliber");
        assert_eq!(escape_lemma("multi word"), "multi_word");
    }

    #[test]
    fn test_entry_xml_id() {
        let poskey = crate::wordnet::PosKey::new("n");
        assert_eq!(entry_xml_id("oewn", "dog", &poskey), "oewn-dog-n");
        assert_eq!(entry_xml_id("oewn", "'hood", &poskey), "oewn--ap-hood-n");
    }

    #[test]
    fn test_synset_xml_id() {
        let id = crate::wordnet::SynsetId::new_owned("00001740-n".to_string());
        assert_eq!(synset_xml_id("oewn", &id), "oewn-00001740-n");
    }

    #[test]
    fn test_sense_xml_id() {
        let id = crate::wordnet::SenseId::new("foot%1:08:00::");
        assert_eq!(sense_xml_id("oewn", &id), "oewn-foot__1.08.00..");
    }

    #[test]
    fn test_strip_prefix_id() {
        assert_eq!(strip_prefix_id("oewn", "oewn-00001740-n"), "00001740-n");
        // No matching prefix: returned unchanged rather than mangled.
        assert_eq!(strip_prefix_id("oewn", "pwn-00001740-n"), "pwn-00001740-n");
    }
}
