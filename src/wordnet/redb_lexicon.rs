#![allow(unused_variables)]
use ouroboros::self_referencing;
use redb::{Database, TableDefinition, ReadableDatabase, ReadableTable, ReadOnlyTable, Range, ReadTransaction, ReadableTableMetadata};
use crate::wordnet::*;
use std::rc::Rc;
use crate::wordnet::entry::BTEntries;
use crate::wordnet::synset::BTSynsets;
use std::collections::HashMap;
use crate::rels::{SenseRelType, SynsetRelType};
use std::borrow::Cow;
use std::path::Path;
use speedy::{Readable, Writable};
use std::result;

const INITIAL_CHARS : [char;27] = ['0', 'a','b', 'c','d','e','f','g','h','i','j','k','l','m','n','o','p','q','r','s','t','u','v','w','x','y','z'];
/// (initial character, lemma) -> HashMap<PosKey, Entry>
const ENTRIES_TABLE: TableDefinition<(char, String), Vec<u8>> = TableDefinition::new("entries");
const LOWERCASE_ENTRIES_TABLE: TableDefinition<String, Vec<String>> = TableDefinition::new("lowercase_entries");
/// (lexname, synset_id) -> Synset
const SYNSETS_TABLE: TableDefinition<(String, String), Vec<u8>> = TableDefinition::new("synsets");
/// (synset_id) -> lexfile
const SYNSET_ID_TO_LEXFILE: TableDefinition<String, String> = TableDefinition::new("synset_id_to_lexfile");
/// (sense_id) -> Vec<(SenseRelType, SenseId)>
const SENSE_LINKS: TableDefinition<String, Vec<u8>> = TableDefinition::new("sense_links");
/// (synset_id) -> Vec<(SynsetRelType, SynsetId)>
const LINKS_TO: TableDefinition<String, Vec<u8>> = TableDefinition::new("links_to");
/// (sense_id) -> (lemma, pos)
const SENSE_ID_TO_LEMMA_POS: TableDefinition<String, (String, String)> = TableDefinition::new("sense_id_to_lemma_pos");
/// DEPRECATION_KEY -> Vec<DeprecationRecord>
const DEPRECATIONS: TableDefinition<&'static str, Vec<u8>> = TableDefinition::new("deprectaions");
const DEPRECATION_KEY:&'static str = "deprecations";

pub struct ReDBLexicon {
    db: Rc<Database>,
    entries: HashMap<char, ReDBEntries>,
    synsets: HashMap<String, ReDBSynsets>,
    lexnames: Vec<String>
}

impl ReDBLexicon {
    /// Load an existing database from disk
    pub fn open<P: AsRef<Path>>(path : P) -> Result<ReDBLexicon> {
        // create database
        //
        let db = Rc::new(Database::open(path)?);
        // Intialize entries as '0' and 'a'..'z'
        //
        let mut entries = HashMap::new();
        for c in INITIAL_CHARS.iter() {
            //
            // Create an entries for each initial character
            //
            entries.insert(*c, ReDBEntries::new(db.clone(), *c));
        }
        // Read all the lexnames from the DB
        //
        let mut synsets = HashMap::new();
        // Assume lexnames is sorted
        let mut lexnames = Vec::new();
        {
            let txn = db.begin_read()?;
            let table = txn.open_table(SYNSETS_TABLE)?;
            for kv in table.iter()? {
                let (lexname, _) = kv?.0.value();
                // Find using binary search
                match lexnames.binary_search(&lexname) {
                    Ok(_) => {}
                    Err(idx) => {
                        lexnames.insert(idx, lexname.clone());
                        synsets.insert(lexname.clone(), ReDBSynsets::new(db.clone(), lexname.clone()));
                    }
                }
            }
        }
        Ok(ReDBLexicon {
            db,
            entries,
            synsets,
            lexnames
        })
    }

    /// Create a new database, deleting the existing file if necessary
    pub fn create<P: AsRef<Path>>(path : P) -> Result<ReDBLexicon> {
        let db = Rc::new(Database::create(path)?);
        // Intialize entries as '0' and 'a'..'z'
        let mut entries = HashMap::new();
        for c in INITIAL_CHARS.iter() {
            //
            // Create an entries for each initial character
            //
            entries.insert(*c, ReDBEntries::new(db.clone(), *c));
        }
        Ok(ReDBLexicon {
            db,
            entries,
            synsets: HashMap::new(),
            lexnames: Vec::new()
        })
    }
}

impl Lexicon for ReDBLexicon {
    type E = ReDBEntries;
    type S = ReDBSynsets;
    // Data access methods
    fn entries_get<'a>(&'a self, key : char) -> Result<Option<Cow<'a, ReDBEntries>>> {
        Ok(self.entries.get(&key).map(|x| Cow::Borrowed(x)))
    }
    fn entries_insert(&mut self, key : char, entries : BTEntries) -> Result<()> {
        for entry in entries.into_entries()? {
            let (lemma, pos, entry) = entry?;
            if let Some(e) = self.entries.get_mut(&key) {
                e.insert_entry(lemma, pos, entry)?;
            }
        }
        Ok(())
    }
    fn entries_iter<'a>(&'a self) -> Result<impl Iterator<Item=Result<(char, Cow<'a, ReDBEntries>)>>> {
        Ok(self.entries.iter().map(|(k, v)| Ok((*k, Cow::Borrowed(v)))))
    }
    fn entries_update<X>(&mut self, key : char, f : impl FnOnce(&mut Self::E) -> X) -> Result<X> {
        if let Some(e) = self.entries.get_mut(&key) {
            Ok(f(e))
        } else {
            let mut e = ReDBEntries::new(self.db.clone(), key);
            let res = f(&mut e);
            self.entries.insert(key, e);
            Ok(res)
        }
    }
    fn synsets_get<'a>(&'a self, lexname : &str) -> Result<Option<Cow<'a, Self::S>>> {
        Ok(self.synsets.get(lexname).map(Cow::Borrowed))
    }
    fn synsets_insert(&mut self, lexname : String, synsets : BTSynsets) -> Result<()> {
        for synset in synsets.into_iter()? {
            let (id, synset) = synset?;
            self.insert_synset(lexname.clone(), id, synset)?;
        }
        Ok(())
    }
    fn synsets_contains_key(&self, lexname : &str) -> Result<bool> {
        Ok(self.lexnames.binary_search(&lexname.to_string()).is_ok())
    }
    fn synsets_iter<'a>(&'a self) -> Result<impl Iterator<Item=Result<(&'a String, Cow<'a, Self::S>)>>> {
        Ok(self.synsets.iter().map(|(k, v)| Ok((k, Cow::Borrowed(v)))))
    }
    fn update_synset(&mut self, synset_id : &SynsetId, f : impl FnOnce(&mut Synset)) -> Result<()> {
        let lexfile_opt = self.synset_id_to_lexfile_get(synset_id)?.map(|x| x.into_owned());
        if let Some(lexfile) = lexfile_opt {
            let res = if let Some(synsets) = self.synsets.get_mut(&lexfile) {
                synsets.update(synset_id, f)?;
                Ok(())
            } else {
                Err(LexiconError::SynsetIdNotFound(synset_id.clone()))
            };
            res
        } else {
            Err(LexiconError::SynsetIdNotFound(synset_id.clone()))
        }
    }
    fn synsets_insert_synset(&mut self, lexname : &str, synset_id : SynsetId, synset : Synset) -> Result<()> {
        let db_clone = self.db.clone();
        self.synsets.entry(lexname.to_owned()).or_insert_with(|| {
            ReDBSynsets::new(db_clone, lexname.to_owned())
        }).insert(lexname.to_owned(), synset_id.clone(), synset.clone())?;
        Ok(())
    }
    fn synsets_remove_synset(&mut self, lexname : &str,  synset_id : &SynsetId) -> Result<Option<(SynsetId, Synset)>> {
        if let Some(synsets) = self.synsets.get_mut(lexname) {
            return synsets.remove(lexname.to_string(), synset_id.clone());
        }
        Ok(None)
    }

    //fn insert_synset(&mut self, lexname : String, synset_id : SynsetId,
    //                     synset : Synset) -> Result<()> {
    //    add_link_to(self, &synset_id, &synset)?;
    //    self.synset_id_to_lexfile_insert(synset_id.clone(), lexname.clone())?;
    //    let db_clone = self.db.clone();
    //    self.synsets.entry(lexname.clone()).or_insert_with(|| {
    //        ReDBSynsets::new(db_clone, lexname.clone())
    //    }).insert(lexname, synset_id, synset)?;
    //    Ok(())
    //}
    //fn remove_synset(&mut self, synset_id : &SynsetId) -> Result<()> {
    //    let lexfile_opt : Option<String> = self.synset_id_to_lexfile_get(synset_id)?.map(|x| x.into_owned());
    //    if let Some(lexfile) = lexfile_opt {
    //        let res = if let Some(synsets) = self.synsets.get_mut(&lexfile) {
    //            synsets.remove_entry(synset_id)?;
    //            Ok(())
    //        } else {
    //            Err(LexiconError::SynsetIdNotFound(synset_id.clone()))
    //        };
    //        res
    //    } else {
    //        Err(LexiconError::SynsetIdNotFound(synset_id.clone()))
    //    }

    //}

    fn synset_id_to_lexfile_get<'a>(&'a self, synset_id : &SynsetId) -> Result<Option<Cow<'a, String>>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(SYNSET_ID_TO_LEXFILE)?;
        if let Some(lexfile) = table.get(synset_id.to_string())? {
            Ok(Some(Cow::Owned(lexfile.value())))
        } else {
            Ok(None)
        }
    }
    fn synset_id_to_lexfile_insert(&mut self, synset_id : SynsetId, lexfile : String) -> Result<()> {
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(SYNSET_ID_TO_LEXFILE)?;
            table.insert(synset_id.to_string(), lexfile)?;
        }
        txn.commit()?;
        Ok(())
    }
    fn sense_links_to_get<'a>(&'a self, sense_id : &SenseId) -> Result<Option<Cow<'a, Vec<(SenseRelType, SenseId)>>>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(SENSE_LINKS)?;
        if let Some(links_str) = table.get(sense_id.to_string())? {
            let links = deserialize_sense_links(links_str.value())?;
            Ok(Some(Cow::Owned(links)))
        } else {
            Ok(None)
        }
    }
    fn sense_links_to_get_or(&mut self, sense_id : SenseId, f : impl FnOnce() -> Vec<(SenseRelType, SenseId)>) -> Result<Vec<(SenseRelType, SenseId)>> {
        let txn = self.db.begin_write()?;
        let table = txn.open_table(SENSE_LINKS)?;
        let mut new_links = None;
        let result = if let Some(links_str) = table.get(sense_id.to_string())? {
            let links = deserialize_sense_links(links_str.value())?;
            Some(links)
        } else {
            let links = f();
            //table.insert(sense_id.to_string(), serialize_sense_links(links.clone())?)?;
            new_links = Some(links);
            None
        };
        match result {
            Some(r) => Ok(r),
            None => {
                let txn = self.db.begin_write()?;
                let result = { 
                    let mut table = txn.open_table(SENSE_LINKS)?;
                    if let Some(links) = new_links {
                        table.insert(sense_id.to_string(), serialize_sense_links(links.clone())?)?;
                        Ok(links)
                    } else {
                        unreachable!()
                    }
                };
                txn.commit()?;
                result
            }
        }
    }

    fn sense_links_to_update(&mut self, sense_id : &SenseId, f : impl FnOnce(&mut Vec<(SenseRelType, SenseId)>)) -> Result<()> {
        let mut links = self.sense_links_to_get_or(sense_id.clone(), || Vec::new())?;
        f(&mut links);
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(SENSE_LINKS)?;
            table.insert(sense_id.to_string(), serialize_sense_links(links)?)?;
        }
        txn.commit()?;
        Ok(())
    }
    fn sense_links_to_push(&mut self, sense_id : SenseId, rel : SenseRelType, target : SenseId) -> Result<()> {
        let mut links = self.sense_links_to_get_or(sense_id.clone(), || Vec::new())?;
        links.push((rel, target));
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(SENSE_LINKS)?;
            table.insert(sense_id.to_string(), serialize_sense_links(links)?)?;
        }
        txn.commit()?;
        Ok(())
    }
    fn set_sense_links_to(&mut self, links_to : HashMap<SenseId, Vec<(SenseRelType, SenseId)>>) -> Result<()> {
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(SENSE_LINKS)?;
            table.retain(|_,_| false)?; // Clear all existing entries
            for (sense_id, links) in links_to {
                table.insert(sense_id.to_string(), serialize_sense_links(links)?)?;
            }
        }
        txn.commit()?;
        Ok(())
    }
    fn links_to_get<'a>(&'a self, synset_id : &SynsetId) -> Result<Option<Cow<'a, Vec<(SynsetRelType, SynsetId)>>>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(LINKS_TO)?;
        if let Some(links_str) = table.get(synset_id.to_string())? {
            let links = deserialize_links(links_str.value())?;
            Ok(Some(Cow::Owned(links)))
        } else {
            Ok(None)
        }
    }
    fn links_to_get_or(&mut self, synset_id : SynsetId, f : impl FnOnce() -> Vec<(SynsetRelType, SynsetId)>) -> Result<Vec<(SynsetRelType, SynsetId)>> {
        let txn = self.db.begin_write()?;
        let table = txn.open_table(LINKS_TO)?;
        let mut new_links = None;
        let result = if let Some(links_str) = table.get(synset_id.to_string())? {
            let links = deserialize_links(links_str.value())?;
            Some(links)
        } else {
            let links = f();
            //table.insert(synset_id.to_string(), serialize_links(links.clone())?)?;
            new_links = Some(links);
            None
        };
        match result {
            Some(r) => Ok(r),
            None => {
                let txn = self.db.begin_write()?;
                let result = { 
                    let mut table = txn.open_table(LINKS_TO)?;
                    if let Some(links) = new_links {
                        table.insert(synset_id.to_string(), serialize_links(links.clone())?)?;
                        Ok(links)
                    } else {
                        unreachable!()
                    }
                };
                txn.commit()?;
                result
            }
        }
    }
    fn links_to_update(&mut self, synset_id : &SynsetId, f : impl FnOnce(&mut Vec<(SynsetRelType, SynsetId)>)) -> Result<()> {
        let mut links = self.links_to_get_or(synset_id.clone(), || Vec::new())?;
        f(&mut links);
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(LINKS_TO)?;
            table.insert(synset_id.to_string(), serialize_links(links)?)?;
        }
        txn.commit()?;
        Ok(())
    }
    fn links_to_push(&mut self, synset_id : SynsetId, rel : SynsetRelType, target : SynsetId) -> Result<()> {
        let mut links = self.links_to_get_or(synset_id.clone(), || Vec::new())?;
        links.push((rel, target));
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(LINKS_TO)?;
            table.insert(synset_id.to_string(), serialize_links(links)?)?;
        }
        txn.commit()?;
        Ok(())
    }
    fn set_links_to(&mut self, links_to : HashMap<SynsetId, Vec<(SynsetRelType, SynsetId)>>) -> Result<()> {
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(LINKS_TO)?;
            table.retain(|_,_| false)?; // Clear all existing entries
            for (synset_id, links) in links_to {
                table.insert(synset_id.to_string(), serialize_links(links)?)?;
            }
        }
        txn.commit()?;
        Ok(())
    }
    fn sense_id_to_lemma_pos_get(&self, sense_id : &SenseId) -> Result<Option<(String, PosKey)>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(SENSE_ID_TO_LEMMA_POS)?;
        if let Some(lemma_pos) = table.get(sense_id.to_string())? {
            let (lemma, pos_str) = lemma_pos.value();
            Ok(Some((lemma.clone(), PosKey::new(pos_str.clone()))))
        } else {
            Ok(None)
        }
    }
    fn sense_id_to_lemma_pos_insert(&mut self, sense_id : SenseId, lemma_pos : (String, PosKey)) -> Result<()> {
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(SENSE_ID_TO_LEMMA_POS)?;
            table.insert(sense_id.to_string(), (lemma_pos.0, lemma_pos.1.as_str().to_string()))?;
        }
        txn.commit()?;
        Ok(())
    }
    fn deprecations_get<'a>(&'a self) -> Result<Cow<'a, Vec<DeprecationRecord>>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(DEPRECATIONS)?;
        if let Some(deprecations_str) = table.get(DEPRECATION_KEY)? {
            let deprecations = deserialize_deprecation(deprecations_str.value())?;
            Ok(Cow::Owned(deprecations))
        } else {
            Ok(Cow::Owned(Vec::new()))
        }
    }
    fn deprecations_push(&mut self, record : DeprecationRecord) -> Result<()> {
        let txn = self.db.begin_write()?;
        { 
            let mut table = txn.open_table(DEPRECATIONS)?;
            let mut deprecations = self.deprecations_get()?.into_owned();
            deprecations.push(record);
            table.insert(DEPRECATION_KEY, serialize_deprecations(deprecations)?)?;
        }
        txn.commit()?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct ReDBEntries {
    db: Rc<Database>,
    key : char
}

impl ReDBEntries {
    fn new(db : Rc<Database>, key : char) -> ReDBEntries {
        ReDBEntries { db, key }
    }
    
    /// Add the lowercase form of the lemma to the lowercase index
    fn register_entry(&mut self, lemma : &str) -> Result<()> {
        let lower_lemma = lemma.to_lowercase();
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(LOWERCASE_ENTRIES_TABLE)?;
            let mut lemmas = if let Some(lemmas_str) = table.get(lower_lemma.clone())? {
                lemmas_str.value()
            } else {
                Vec::new()
            };
            if !lemmas.contains(&lemma.to_string()) {
                lemmas.push(lemma.to_string());
                table.insert(lower_lemma, lemmas)?;
            }
        }
        txn.commit()?;
        Ok(())
    }
}


impl Entries for ReDBEntries {
    fn entry<'a>(&'a self, lemma : &str, pos_key : &PosKey) -> Result<Option<Cow<'a, Entry>>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(ENTRIES_TABLE)?;
        if let Some(entry_str) = table.get((self.key, lemma.to_string()))? {
            let entry_map = deserialize_entry(entry_str.value())?;
            if let Some(entry) = entry_map.get(pos_key) {
                Ok(Some(Cow::Owned(entry.clone())))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
    fn insert_entry(&mut self, lemma : String, pos : PosKey, entry : Entry) -> Result<()> {
        self.register_entry(&lemma)?;
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(ENTRIES_TABLE)?;
            let mut entry_map = if let Some(entry_str) = table.get((self.key, lemma.clone()))? {
                deserialize_entry(entry_str.value())?
            } else {
                HashMap::new()
            };
            entry_map.insert(pos, entry);
            table.insert((self.key, lemma), serialize_entry(&entry_map)?)?;
        }
        txn.commit()?;
        Ok(())
    }
    fn update_entry<X>(&mut self, lemma : &str, pos_key : &PosKey,
        f : impl FnOnce(&mut Entry) -> X) -> Result<X> {
        let txn = self.db.begin_write()?;
        let result = {
            let mut table = txn.open_table(ENTRIES_TABLE)?;
            let mut entry_map = if let Some(entry_str) = table.get((self.key, lemma.to_string()))? {
                deserialize_entry(entry_str.value())?
            } else {
                return Err(LexiconError::EntryNotFound(lemma.to_string(), pos_key.clone()));
            };
            if let Some(mut entry) = entry_map.get_mut(pos_key) {
                let res = f(&mut entry);
                table.insert((self.key, lemma.to_string()), serialize_entry(&entry_map)?)?;
                Ok(res)
            } else {
                return Err(LexiconError::EntryNotFound(lemma.to_string(), pos_key.clone()));
            }
        };
        txn.commit()?;
        result
    }

    fn remove_entry(&mut self, lemma : &str, pos_key : &PosKey) -> Result<Option<Entry>> {
        // Note: we don't deregister lemmas, which could lead to some DB bloat over time
        let txn = self.db.begin_write()?;
        let result = {
            let mut table = txn.open_table(ENTRIES_TABLE)?;
            let mut entry_map = if let Some(entry_str) = table.get((self.key, lemma.to_string()))? {
                deserialize_entry(entry_str.value())?
            } else {
                return Ok(None);
            };
            let removed_entry = entry_map.remove(pos_key);
            if entry_map.is_empty() {
                table.remove((self.key, lemma.to_string()))?;
            } else {
                table.insert((self.key, lemma.to_string()), serialize_entry(&entry_map)?)?;
            }
            Ok(removed_entry)
        };
        txn.commit()?;
        result
    }

    fn entry_by_lemma<'a>(&'a self, lemma : &str) -> Result<Vec<Cow<'a, Entry>>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(ENTRIES_TABLE)?;
        if let Some(entry_str) = table.get((self.key, lemma.to_string()))? {
            let entry_map = deserialize_entry(entry_str.value())?;
            Ok(entry_map.into_iter().map(|(_,e)| Cow::Owned(e)).collect())
        } else {
            Ok(Vec::new())
        }
    }
    fn entry_by_lemma_with_pos<'a>(&'a self, lemma : &str) -> Result<Vec<(PosKey, Cow<'a, Entry>)>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(ENTRIES_TABLE)?;
        if let Some(entry_str) = table.get((self.key, lemma.to_string()))? {
            let entry_map = deserialize_entry(entry_str.value())?;
            Ok(entry_map.into_iter().map(|(p,e)| (p, Cow::Owned(e))).collect())
        } else {
            Ok(Vec::new())
        }
    }
    fn entry_by_lemma_ignore_case<'a>(&'a self, lemma : &str) -> Result<Vec<Cow<'a, Entry>>> {
        let lemmas = {
            let txn = self.db.begin_read()?;
            let table = txn.open_table(LOWERCASE_ENTRIES_TABLE)?;
            if let Some(lemmas_str) = table.get(lemma.to_lowercase())? {
                lemmas_str.value()
            } else {
                Vec::new()
            }
        };
        let mut results = Vec::new();
        for l in lemmas {
            let entries = self.entry_by_lemma(&l)?;
            results.extend(entries);
        }
        Ok(results)
    }

    fn entries<'a>(&'a self) -> Result<impl Iterator<Item=Result<(String, PosKey, Cow<'a, Entry>)>>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(ENTRIES_TABLE)?;
        Ok(EntryIterator::new(txn, table, |table| {
            let next_char = std::char::from_u32(self.key as u32 + 1).expect("Impossible as we are only using ASCII");
            table.range((self.key,"".to_string())..(next_char,"".to_string()))
                .map_err(|e| e.to_string())
        }).flat_map(|e| {
            let it = match e {
                Ok((l,dict)) => Box::new(dict.into_iter().map(move |(p,e)| {
                    Ok((l.clone(), p, Cow::Owned(e)))
                })),
                Err(err) => Box::new(single_item_iterator(Err(err))) as Box<dyn Iterator<Item=_>>
            };
            it
        }))
    }

    fn into_entries(self) -> Result<impl Iterator<Item=Result<(String, PosKey, Entry)>>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(ENTRIES_TABLE)?;
        Ok(EntryIterator::new(txn, table, |table| {
            let next_char = std::char::from_u32(self.key as u32 + 1).expect("Impossible as we are only using ASCII");
            table.range((self.key,"".to_string())..(next_char,"".to_string()))
                .map_err(|e| e.to_string())
        }).flat_map(|e| {
            let it = match e {
                Ok((l,dict)) => Box::new(dict.into_iter().map(move |(p,e)| {
                    Ok((l.clone(), p, e.clone()))
                })),
                Err(err) => Box::new(single_item_iterator(Err(err))) as Box<dyn Iterator<Item=_>>
            };
            it
        }))
    }

    fn n_entries(&self) -> Result<usize> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(ENTRIES_TABLE)?;
        Ok(table.len()? as usize)
    }
 
}

#[derive(Clone)]
pub struct ReDBSynsets {
    db: Rc<Database>,
    lexname : String
}

impl ReDBSynsets {
    pub fn new(db : Rc<Database>, lexname : String) -> ReDBSynsets {
        ReDBSynsets { db, lexname }
    }

    fn insert(&mut self, lexname : String, synset_id : SynsetId,
                         synset : Synset) -> Result<()> {
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(SYNSETS_TABLE)?;
            table.insert((lexname, synset_id.to_string()), serialize_synset(&synset)?)?;
        }
        txn.commit()?;
        Ok(())
    }

    fn remove(&mut self, lexname : String, synset_id : SynsetId) -> Result<Option<(SynsetId, Synset)>> {
        let txn = self.db.begin_write()?;
        let result = {
            let mut table = txn.open_table(SYNSETS_TABLE)?;
            let res = if let Some(s) = table.remove((lexname, synset_id.to_string()))? {
                let synset = deserialize_synset(s.value())?;
                Ok(Some((synset_id, synset)))
            } else {
                Ok(None)
            };
            res
        };
        txn.commit()?;
        result
    }

    fn update<X>(&mut self, id : &SynsetId, f : impl FnOnce(&mut Synset) -> X) -> Result<X> {
        let txn = self.db.begin_write()?;
        let (result, synset) = {
            let table = txn.open_table(SYNSETS_TABLE)?;
            let x = {
                if let Some(synset_str) = table.get((self.lexname.clone(), id.to_string()))? {
                    let mut synset = deserialize_synset(synset_str.value())?;
                    let res = f(&mut synset);
                    (Ok(res), synset)
                } else {
                    return Err(LexiconError::SynsetIdNotFound(id.clone()));
                }
            };
            x
        };
        txn.commit()?;
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(SYNSETS_TABLE)?;
            table.insert((self.lexname.clone(), id.to_string()), serialize_synset(&synset)?)?;
        }
        result
    }
 }

impl Synsets for ReDBSynsets {
    fn get<'a>(&'a self, id : &SynsetId) -> Result<Option<Cow<'a, Synset>>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(SYNSETS_TABLE)?;
        if let Some(synset_str) = table.get((self.lexname.clone(), id.to_string()))? {
            let synset = deserialize_synset(synset_str.value())?;
            Ok(Some(Cow::Owned(synset)))
        } else {
            Ok(None)
        }
    }
   fn iter<'a>(&'a self) -> Result<impl Iterator<Item=Result<(SynsetId, Cow<'a, Synset>)>> + 'a> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(SYNSETS_TABLE)?;
        Ok(SynsetIterator::new(txn, table, |table| {
            let next_string = format!("{}a", self.lexname);
            table.range((self.lexname.clone(),"".to_string())..(next_string,"".to_string()))
                .map_err(|e| e.to_string())
        }).map(|kv| {
            let (k, v) = kv?;
            Ok((k, Cow::Owned(v)))
        }))
    }
    fn into_iter(self) -> Result<impl Iterator<Item=Result<(SynsetId, Synset)>>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(SYNSETS_TABLE)?;
        Ok(SynsetIterator::new(txn, table, |table| {
            table.iter().map_err(|e| e.to_string())
        }).map(|kv| {
            let (k, v) = kv?;
            Ok((k, v))
        }))
    }
    fn len(&self) -> Result<usize> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(SYNSETS_TABLE)?;
        Ok(table.len()? as usize)
    }
    fn remove_entry(&mut self, id : &SynsetId) -> Result<Option<(SynsetId, Synset)>> {
        let txn = self.db.begin_write()?;
        let result = {
            let mut table = txn.open_table(SYNSETS_TABLE)?;
            let r = table.remove((self.lexname.clone(), id.to_string()))?;
            if let Some(synset_str) = r {
                let synset = deserialize_synset(synset_str.value())?;
                Ok(Some((id.clone(), synset)))
            } else {
                Ok(None)
            }
        };
        txn.commit()?;
        result
    }
}

#[self_referencing]
pub struct SynsetIterator {
    txn: ReadTransaction,
    table: ReadOnlyTable<(String, String), Vec<u8>>,
    #[borrows(table)]
    #[covariant]
    inner: result::Result<Range<'this, (String, String), Vec<u8>>, String>
}

impl Iterator for SynsetIterator {
    type Item = Result<(SynsetId, Synset)>;

    fn next(&mut self) -> Option<Self::Item> {
        self.with_inner_mut(|inner| {
            match inner {
                Ok(inner) => {
                    inner.next().map(|res| {
                        let (k, v) = res?;
                        Ok((SynsetId::new_owned(k.value().1), deserialize_synset(v.value())?))
                    })
                },
                Err(e) => Some(Err(LexiconError::GenericError(e.clone())))
            }
        })
    }
}


#[self_referencing]
pub struct EntryIterator {
    txn: ReadTransaction,
    table: ReadOnlyTable<(char, String), Vec<u8>>,
    #[borrows(table)]
    #[covariant]
    inner: result::Result<Range<'this, (char, String), Vec<u8>>, String>
}

impl Iterator for EntryIterator {
    type Item = Result<(String, HashMap<PosKey, Entry>)>;

    fn next(&mut self) -> Option<Self::Item> {
        self.with_inner_mut(|inner| {
            match inner {
                Ok(inner) => inner.next().map(|res| {
                    let (ks, v) = res?;
                    let (k1, k2) = ks.value();
                    Ok((k2,
                        deserialize_entry(v.value())?))
                }),
                Err(e) => Some(Err(LexiconError::GenericError(e.clone())))
            }
        })
    }
}

fn deserialize_synset(s : Vec<u8>) -> Result<Synset> {
   Ok(Synset::read_from_buffer(&s)?)
}

fn serialize_synset(synset : &Synset) -> Result<Vec<u8>> {
    Ok(synset.write_to_vec()?)
}

fn deserialize_entry(s : Vec<u8>) -> Result<HashMap<PosKey, Entry>> {
    Ok(HashMap::<PosKey, Entry>::read_from_buffer(&s)?)
}

fn serialize_entry(entry : &HashMap<PosKey, Entry>) -> Result<Vec<u8>> {
    Ok(entry.write_to_vec()?)
}

fn single_item_iterator<T>(t : T) -> impl Iterator<Item=T> {
    Some(t).into_iter()
}

fn deserialize_sense_links(s : Vec<u8>) -> Result<Vec<(SenseRelType, SenseId)>> {
    Ok(Vec::read_from_buffer(&s)?)
}

fn serialize_sense_links(links : Vec<(SenseRelType, SenseId)>) -> Result<Vec<u8>> {
    Ok(links.write_to_vec()?)
}

fn deserialize_links(s : Vec<u8>) -> Result<Vec<(SynsetRelType, SynsetId)>> {
    Ok(Vec::read_from_buffer(&s)?)
}

fn serialize_links(links : Vec<(SynsetRelType, SynsetId)>) -> Result<Vec<u8>> {
    Ok(links.write_to_vec()?)
}

fn deserialize_deprecation(s : Vec<u8>) -> Result<Vec<DeprecationRecord>> {
    Ok(Vec::read_from_buffer(&s)?)
}

fn serialize_deprecations(deprecations : Vec<DeprecationRecord>) -> Result<Vec<u8>> {
    Ok(deprecations.write_to_vec()?)
}
