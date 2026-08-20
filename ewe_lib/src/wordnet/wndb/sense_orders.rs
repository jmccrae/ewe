//! `--sense-orders` CSV loading (`lemma,pos,synset_id1 synset_id2 ...`) - see the
//! `wndb-export-sense-orders-csv` memory note for why this is needed: when two differently-cased
//! YAML entries (e.g. `afghani`/`Afghani`) fold into a single `index.{pos}` line, there's no
//! inherent ordering across their two separate sense lists once merged, so this file records one.

use super::WndbExportError;
use crate::wordnet::SynsetId;
use std::collections::HashMap;
use std::path::Path;

/// lowercased-lemma-as-written-in-the-CSV -> ordered synset ids (each `{numeric-id}-{pos}`, no
/// lexicon id-prefix - the CSV's own `"oewn-" + id + "-" + pos` construction is irrelevant here
/// since `ewe_lib`'s `SynsetId` never carries a prefix at all).
pub(super) type SenseOrders = HashMap<String, Vec<SynsetId>>;

pub(super) fn load(path: Option<&Path>) -> std::result::Result<SenseOrders, WndbExportError> {
    let mut map = SenseOrders::new();
    let Some(path) = path else {
        return Ok(map);
    };
    let mut reader = csv::ReaderBuilder::new().has_headers(false).from_path(path)?;
    for record in reader.records() {
        let record = record?;
        if record.len() < 3 {
            continue;
        }
        let lemma = record[0].to_string();
        let pos = &record[1];
        let ids: Vec<SynsetId> = record[2]
            .split(' ')
            .filter(|s| !s.is_empty())
            .map(|id| SynsetId::new(&format!("{id}-{pos}")))
            .collect();
        map.insert(lemma, ids);
    }
    Ok(map)
}
