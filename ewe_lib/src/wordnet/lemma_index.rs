//! In-memory index for fuzzy lemma search.
//!
//! Lemmas are normalized (camelCase split, diacritics removed, lower-cased, separators
//! ignored) and matched in tiers: exact, prefix, word-prefix (infix words of multiword
//! lemmas) and finally typo-tolerant matching via a trigram index verified with an edit
//! distance. The index is derived from the live entries, so it never contains stale lemmas.
use crate::wordnet::entry::Entries;
use crate::wordnet::lexicon::{Lexicon, Result};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

/// Queries shorter than this (in normalized characters) are never fuzzy matched
const MIN_FUZZY_LEN: usize = 3;
/// Maximum edit distance for queries of up to `SHORT_QUERY_LEN` characters
const SHORT_QUERY_LEN: usize = 5;

/// Normalize a lemma or query into lower-case, accent-free words
pub(crate) fn normalize_words(s: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut cur = String::new();
    let mut prev_lower = false;
    for c in s.nfd() {
        if is_combining_mark(c) {
            continue;
        }
        if !c.is_alphanumeric() {
            if !cur.is_empty() {
                words.push(std::mem::take(&mut cur));
            }
            prev_lower = false;
            continue;
        }
        if c.is_uppercase() && prev_lower && !cur.is_empty() {
            words.push(std::mem::take(&mut cur));
        }
        prev_lower = c.is_lowercase();
        cur.extend(c.to_lowercase());
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    words
}

fn max_distance(len: usize) -> usize {
    if len <= SHORT_QUERY_LEN {
        1
    } else {
        2
    }
}

/// Optimal string alignment distance (Levenshtein plus adjacent transpositions)
fn edit_distance(a: &[char], b: &[char]) -> usize {
    let mut d = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for i in 0..=a.len() {
        d[i][0] = i;
    }
    for j in 0..=b.len() {
        d[0][j] = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            d[i][j] = (d[i - 1][j] + 1).min(d[i][j - 1] + 1).min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d[i][j] = d[i][j].min(d[i - 2][j - 2] + 1);
            }
        }
    }
    d[a.len()][b.len()]
}

fn trigrams(s: &str) -> Vec<[char; 3]> {
    let cs: Vec<char> = std::iter::once('^')
        .chain(s.chars())
        .chain(std::iter::once('$'))
        .collect();
    cs.windows(3).map(|w| [w[0], w[1], w[2]]).collect()
}

pub struct LemmaIndex {
    lemmas: Vec<String>,
    words: Vec<Vec<String>>,
    compact: Vec<String>,
    /// Lemma ids sorted by compact form
    by_compact: Vec<u32>,
    /// (word, lemma id) sorted by word
    by_word: Vec<(String, u32)>,
    trigrams: HashMap<[char; 3], Vec<u32>>,
}

impl LemmaIndex {
    /// Build the index from the lemmas in a lexicon
    pub fn build<L: Lexicon>(lexicon: &L) -> Result<LemmaIndex> {
        let mut seen = HashSet::new();
        let mut all = Vec::new();
        for section in lexicon.entries_iter()? {
            let (_, entries) = section?;
            for e in entries.entries()? {
                let (lemma, _, _) = e?;
                if seen.insert(lemma.clone()) {
                    all.push(lemma);
                }
            }
        }
        Ok(Self::from_lemmas(all))
    }

    pub fn from_lemmas(lemmas: Vec<String>) -> LemmaIndex {
        let words: Vec<Vec<String>> = lemmas.iter().map(|l| normalize_words(l)).collect();
        let compact: Vec<String> = words.iter().map(|w| w.concat()).collect();
        let mut by_compact: Vec<u32> = (0..lemmas.len() as u32).collect();
        by_compact.sort_by(|a, b| compact[*a as usize].cmp(&compact[*b as usize]));
        let mut by_word = Vec::new();
        let mut tri: HashMap<[char; 3], Vec<u32>> = HashMap::new();
        for (i, ws) in words.iter().enumerate() {
            for w in ws {
                by_word.push((w.clone(), i as u32));
            }
            let mut ts = trigrams(&compact[i]);
            ts.sort();
            ts.dedup();
            for t in ts {
                tri.entry(t).or_default().push(i as u32);
            }
        }
        by_word.sort();
        LemmaIndex { lemmas, words, compact, by_compact, by_word, trigrams: tri }
    }

    pub fn len(&self) -> usize {
        self.lemmas.len()
    }

    /// Find lemmas matching a query, best matches first
    pub fn search(&self, query: &str, limit: usize) -> Vec<String> {
        let qwords = normalize_words(query);
        let qc = qwords.concat();
        if qc.is_empty() || limit == 0 {
            return Vec::new();
        }
        let mut seen: HashSet<u32> = HashSet::new();
        let mut out: Vec<u32> = Vec::new();

        // Tier 1 + 2: exact then prefix on the compact form
        let start = self.by_compact.partition_point(|i| self.compact[*i as usize].as_str() < qc.as_str());
        let mut exact = Vec::new();
        let mut prefix = Vec::new();
        for i in &self.by_compact[start..] {
            let c = &self.compact[*i as usize];
            if *c == qc {
                exact.push(*i);
            } else if c.starts_with(&qc) {
                prefix.push(*i);
            } else {
                break;
            }
        }
        self.push_tier(exact, &mut seen, &mut out);
        self.push_tier(prefix, &mut seen, &mut out);

        // Tier 3: every query word is the prefix of some word of the lemma
        if out.len() < limit {
            let first = &qwords[0];
            let s = self.by_word.partition_point(|(w, _)| w.as_str() < first.as_str());
            let mut cands = Vec::new();
            for (w, i) in &self.by_word[s..] {
                if !w.starts_with(first.as_str()) {
                    break;
                }
                if qwords[1..]
                    .iter()
                    .all(|q| self.words[*i as usize].iter().any(|lw| lw.starts_with(q.as_str())))
                {
                    cands.push(*i);
                }
            }
            self.push_tier(cands, &mut seen, &mut out);
        }

        // Tier 4: typos
        let qlen = qc.chars().count();
        if out.len() < limit && qlen >= MIN_FUZZY_LEN {
            let maxd = max_distance(qlen);
            let qchars: Vec<char> = qc.chars().collect();
            let mut qts = trigrams(&qc);
            qts.sort();
            qts.dedup();
            let mut counts: HashMap<u32, usize> = HashMap::new();
            for t in &qts {
                if let Some(ids) = self.trigrams.get(t) {
                    for i in ids {
                        *counts.entry(*i).or_default() += 1;
                    }
                }
            }
            // Each edit can destroy at most three trigrams
            let needed = qts.len().saturating_sub(3 * maxd).max(1);
            let mut cands: Vec<(usize, u32)> = Vec::new();
            for (i, n) in counts {
                if n < needed || seen.contains(&i) {
                    continue;
                }
                let c: Vec<char> = self.compact[i as usize].chars().collect();
                if c.len().abs_diff(qchars.len()) > maxd {
                    continue;
                }
                let d = edit_distance(&qchars, &c);
                if d <= maxd {
                    cands.push((d, i));
                }
            }
            cands.sort_by(|a, b| {
                a.0.cmp(&b.0)
                    .then(self.lemmas[a.1 as usize].len().cmp(&self.lemmas[b.1 as usize].len()))
                    .then(self.lemmas[a.1 as usize].cmp(&self.lemmas[b.1 as usize]))
            });
            for (_, i) in cands {
                if seen.insert(i) {
                    out.push(i);
                }
            }
        }
        out.into_iter().take(limit).map(|i| self.lemmas[i as usize].clone()).collect()
    }

    fn push_tier(&self, mut ids: Vec<u32>, seen: &mut HashSet<u32>, out: &mut Vec<u32>) {
        ids.sort_by(|a, b| {
            self.lemmas[*a as usize]
                .len()
                .cmp(&self.lemmas[*b as usize].len())
                .then(self.lemmas[*a as usize].cmp(&self.lemmas[*b as usize]))
        });
        for i in ids {
            if seen.insert(i) {
                out.push(i);
            }
        }
    }
}

/// Lazily built, invalidatable holder for a `LemmaIndex`. Cloning yields an empty cache.
#[derive(Default)]
pub struct LemmaIndexCache(RwLock<Option<Arc<LemmaIndex>>>);

impl Clone for LemmaIndexCache {
    fn clone(&self) -> Self {
        LemmaIndexCache::default()
    }
}

impl LemmaIndexCache {
    pub fn invalidate(&self) {
        if let Ok(mut g) = self.0.write() {
            *g = None;
        }
    }

    pub fn get_or_build(&self, build: impl FnOnce() -> Result<LemmaIndex>) -> Result<Arc<LemmaIndex>> {
        if let Some(i) = self.0.read().ok().and_then(|g| g.clone()) {
            return Ok(i);
        }
        let idx = Arc::new(build()?);
        if let Ok(mut g) = self.0.write() {
            *g = Some(idx.clone());
        }
        Ok(idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idx() -> LemmaIndex {
        LemmaIndex::from_lemmas(
            ["café", "Paris", "ice-cream", "ice cream", "whipped cream", "accommodate", "cream", "apple", "cat"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        )
    }

    #[test]
    fn test_normalize() {
        assert_eq!(normalize_words("iceCream"), vec!["ice", "cream"]);
        assert_eq!(normalize_words("Café_au-lait"), vec!["cafe", "au", "lait"]);
    }

    #[test]
    fn test_matching() {
        let i = idx();
        assert_eq!(i.search("cafe", 10), vec!["café"]);
        assert_eq!(i.search("paris", 10), vec!["Paris"]);
        assert_eq!(i.search("iceCream", 10), vec!["ice cream", "ice-cream"]);
        assert_eq!(i.search("icecream", 10), vec!["ice cream", "ice-cream"]);
        assert_eq!(i.search("accomodate", 10), vec!["accommodate"]);
        assert_eq!(i.search("accommodate", 10), vec!["accommodate"]);
        assert!(i.search("zzzzzz", 10).is_empty());
        assert!(i.search("", 10).is_empty());
        assert!(i.search("---", 10).is_empty());
    }

    #[test]
    fn test_ranking_and_limit() {
        let i = idx();
        // exact, then word matches
        assert_eq!(i.search("cream", 10), vec!["cream", "ice cream", "ice-cream", "whipped cream"]);
        assert_eq!(i.search("cream", 2), vec!["cream", "ice cream"]);
        // prefix before word match before fuzzy
        let r = i.search("ca", 10);
        assert_eq!(&r[..2], &["cat".to_string(), "café".to_string()]);
    }

    #[test]
    fn test_transposition() {
        assert_eq!(idx().search("appel", 10), vec!["apple"]);
    }
}
