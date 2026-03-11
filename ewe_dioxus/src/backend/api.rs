use dioxus::prelude::*;
use oewn_lib::wordnet::{SynsetId, MemberSynset, Lexicon};
use thiserror::Error;

#[derive(Error, Debug)]
enum EweAPIError {
    #[error("Lexicon not available")]
    LexiconUnavailable,
}

#[get("/api/lemma/{lemma}")]
pub async fn get_lemma(lemma: String) -> Result<Vec<SynsetId>> {
    if let Some(lexicon) = crate::LEXICON.get() {
        let lemmas = lexicon.entry_by_lemma(&lemma)?;
        let synset_ids = lemmas.iter().flat_map(|entry| entry.sense.iter().map(|sense| sense.synset.clone())).collect();
        Ok(synset_ids)
    } else {
        Err(EweAPIError::LexiconUnavailable.into())
    }
}

#[get("/api/autocomplete/{query}")]
pub async fn autocomplete(query: String) -> Result<Vec<String>> {
    if let Some(lexicon) = crate::LEXICON.get() {
        let mut results = Vec::new();
        results.extend(lexicon.lemma_by_prefix(&query)?);
        //results.extend(lexicon.ili_by_prefix(&query));
        results.extend(lexicon.ssid_by_prefix(&query)?);
        results.sort_by(|a, b| {
            match a.to_lowercase().cmp(&b.to_lowercase()) {
                std::cmp::Ordering::Equal => a.cmp(b).reverse(),
                x => x
            }
        });
        Ok(results)
    } else {
        Err(EweAPIError::LexiconUnavailable.into())
    }
}

#[get("/api/synset/{id}")]
pub async fn get_synset(id: SynsetId) -> Result<Option<MemberSynset>> {
    if let Some(lexicon) = crate::LEXICON.get() {
        let synset = lexicon.synset_by_id(&id)?;
        if let Some(synset) = synset {
            Ok(Some(MemberSynset::from_synset(&id, synset.into_owned(), lexicon)?))
        } else {
            Ok(None)
        }
    } else {
        Err(EweAPIError::LexiconUnavailable.into())
    }
}
