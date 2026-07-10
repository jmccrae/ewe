//! API endpoints for looking up where a sense occurs in the Semcor corpus.

use dioxus::prelude::*;
use oewn_lib::wordnet::SynsetId;
use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use teanga::query::QueryBuilder;
#[allow(unused_imports)]
use teanga::{Corpus, Layer, ReadableCorpus};
use thiserror::Error;

/// The corpus layer that carries sense annotations, and the one that
/// `db::open_corpus` builds a search index for. Shared as a constant so the
/// indexed layer name and the queried layer name can never drift apart.
#[allow(dead_code)]
pub(crate) const OEWN_KEY_LAYER: &str = "oewn_key";

#[derive(Error, Debug)]
#[allow(dead_code)]
enum EweSensesError {
    #[error("Semcor corpus not available")]
    CorpusUnavailable,
}

/// How many tokens of context to keep on each side of a match. The senses
/// view clips this to one line with CSS (`overflow: hidden`), so this only
/// needs to comfortably fill that line at any reasonable column width - it
/// doesn't need to track sentence boundaries precisely.
#[allow(dead_code)]
const CONTEXT_WORDS: usize = 20;

/// The Semcor `oewn_key` layer stores sense keys with an `oewn-` prefix
/// (e.g. `oewn-00001740-n`), unlike the bare synset ids used elsewhere in
/// this app.
#[allow(dead_code)]
fn oewn_key(id: &SynsetId) -> String {
    format!("oewn-{}", id.as_str())
}

/// A token can carry more than one sense key, joined by `;`, when the
/// annotators judged more than one sense applicable.
#[allow(dead_code)]
fn has_sense(value: &str, target: &str) -> bool {
    value.split(';').any(|part| part == target)
}

/// A single occurrence of a sense in the corpus, with enough surrounding
/// text to render as a keyword-in-context (KWIC) row.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConcordanceLine {
    pub doc_id: String,
    pub left: String,
    pub target: String,
    pub right: String,
}

/// Rows per page in [`get_sense_concordance`].
#[allow(dead_code)]
pub(crate) const PAGE_SIZE: usize = 100;

/// One page of [`get_sense_concordance`] results.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConcordancePage {
    pub lines: Vec<ConcordanceLine>,
    /// 0-indexed.
    pub page: usize,
    pub total_pages: usize,
    pub total: usize,
}

/// Every occurrence of the given synset id in the corpus, one row per
/// tagged token (a document that uses a sense more than once contributes
/// more than one row).
///
/// Candidate documents come from `corpus.search`, which uses the index
/// `db::open_corpus` builds on [`OEWN_KEY_LAYER`] to avoid scanning every
/// document. That index (like the query it serves) matches a token's
/// `oewn_key` value exactly, so a document whose *only* occurrence of this
/// sense is packed into a `;`-joined multi-key token (see [`has_sense`])
/// won't be found here even though the sense is technically present. Once a
/// document *is* found this way, every occurrence within it - combined keys
/// included - is still picked up correctly by the per-document scan below.
#[cfg(feature = "server")]
#[allow(dead_code)]
async fn all_concordance_lines(id: SynsetId) -> Result<Vec<ConcordanceLine>> {
    if let Some(corpus) = crate::CORPUS.get() {
        let target = oewn_key(&id);
        let meta = corpus.get_meta();
        let query = QueryBuilder::new().value(OEWN_KEY_LAYER, target.clone()).build();
        let mut lines = Vec::new();
        for result in corpus.search(query) {
            let (doc_id, doc) = result?;
            let Some(Layer::L1S(pairs)) = doc.get(OEWN_KEY_LAYER) else {
                continue;
            };
            let matches: Vec<usize> = pairs
                .iter()
                .filter(|(_, value)| has_sense(value, &target))
                .map(|(idx, _)| *idx as usize)
                .collect();
            if matches.is_empty() {
                continue;
            }
            let Ok(tokens) = doc.text("tokens", meta) else {
                continue;
            };
            for i in matches {
                let Some(&target_word) = tokens.get(i) else {
                    continue;
                };
                let left_start = i.saturating_sub(CONTEXT_WORDS);
                let right_end = (i + 1 + CONTEXT_WORDS).min(tokens.len());
                lines.push(ConcordanceLine {
                    doc_id: doc_id.clone(),
                    left: tokens[left_start..i].join(" "),
                    target: target_word.to_string(),
                    right: tokens[i + 1..right_end].join(" "),
                });
            }
        }
        Ok(lines)
    } else {
        Err(EweSensesError::CorpusUnavailable.into())
    }
}

/// `page` is 0-indexed; out-of-range pages clamp to the last page rather
/// than erroring.
///
/// Takes `page` as a path segment rather than `?{page}` query parameter:
/// dioxus-fullstack 0.7.9's `Query` extractor (backed by `serde_qs`) fails
/// to deserialize a single-field `Option<T>` query struct - reproducible
/// even on the pre-existing `autocomplete` endpoint's `max_results` param,
/// so it's an upstream issue, not something specific to this route.
#[allow(dead_code)]
#[get("/api/senses/{id}/concordance/{page}")]
pub async fn get_sense_concordance(id: SynsetId, page: usize) -> Result<ConcordancePage> {
    let all = all_concordance_lines(id).await?;
    let total = all.len();
    let total_pages = total.saturating_sub(1) / PAGE_SIZE + 1;
    let page = page.min(total_pages - 1);
    let start = page * PAGE_SIZE;
    let end = (start + PAGE_SIZE).min(total);
    Ok(ConcordancePage {
        lines: all[start..end].to_vec(),
        page,
        total_pages,
        total,
    })
}

/// A fast (index-backed), approximate count of how many documents contain
/// at least one occurrence of the given synset id - intended for "is this
/// sense worth linking to?" checks, not as an exact occurrence count.
/// Subject to the same `;`-joined-key blind spot as [`get_sense_concordance`],
/// so it can under-count, but a positive result always means real
/// annotations exist.
#[allow(dead_code)]
#[get("/api/senses/{id}/count")]
pub async fn get_sense_count(id: SynsetId) -> Result<usize> {
    if let Some(corpus) = crate::CORPUS.get() {
        let query = QueryBuilder::new()
            .value(OEWN_KEY_LAYER, oewn_key(&id))
            .build();
        Ok(corpus.estimate_query_count(query)?)
    } else {
        Err(EweSensesError::CorpusUnavailable.into())
    }
}

/// The ids of the documents that contain at least one token tagged with the
/// given synset id.
#[get("/api/senses/{id}")]
pub async fn get_sense_documents(id: SynsetId) -> Result<Vec<String>> {
    let mut doc_ids: Vec<String> = all_concordance_lines(id)
        .await?
        .into_iter()
        .map(|line| line.doc_id)
        .collect();
    doc_ids.sort();
    doc_ids.dedup();
    Ok(doc_ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oewn_key() {
        let id = SynsetId::new_owned("00001740-n".to_string());
        assert_eq!(oewn_key(&id), "oewn-00001740-n");
    }

    #[test]
    fn test_has_sense_single() {
        assert!(has_sense("oewn-00001740-n", "oewn-00001740-n"));
        assert!(!has_sense("oewn-00001740-n", "oewn-00001741-n"));
    }

    #[test]
    fn test_has_sense_combined() {
        let value = "oewn-00029976-n;oewn-01025762-n";
        assert!(has_sense(value, "oewn-00029976-n"));
        assert!(has_sense(value, "oewn-01025762-n"));
        assert!(!has_sense(value, "oewn-00000000-n"));
    }
}
