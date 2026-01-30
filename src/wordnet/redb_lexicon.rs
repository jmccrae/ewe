#![allow(unused_variables)]
use ouroboros::self_referencing;
use redb::{Database, TableDefinition, ReadableDatabase, ReadableTable, ReadOnlyTable, Range, ReadTransaction};
use crate::wordnet::*;
use std::rc::Rc;
use crate::wordnet::entry::BTEntries;
use crate::wordnet::synset::BTSynsets;
use std::collections::HashMap;
use crate::rels::{SenseRelType, SynsetRelType};
use std::borrow::Cow;

const ENTRIES_TABLE: TableDefinition<(String, String), String> = TableDefinition::new("entries");
const SYNSETS_TABLE: TableDefinition<String, String> = TableDefinition::new("synsets");


pub struct ReDBLexicon {
    db: Rc<Database>,
    entries: HashMap<String, ReDBEntries>,
    synsets: HashMap<String, ReDBSynsets>,
}

impl Lexicon for ReDBLexicon {
    type E = ReDBEntries;
    type S = ReDBSynsets;
    // Data access methods
    fn entries_get<'a>(&'a self, lemma : &str) -> Option<Cow<'a, ReDBEntries>> {
        self.entries.get(lemma).map(|x| Cow::Borrowed(x))
    }
    fn entries_insert(&mut self, key : String, entries : BTEntries) {
        for (lemma, pos, entry) in entries.into_entries() {
            if let Some(e) = self.entries.get_mut(&key) {
                e.insert_entry(lemma, pos, entry);
            }
        }
    }
    fn entries_iter<'a>(&'a self) -> impl Iterator<Item=(&'a String, Cow<'a, ReDBEntries>)> {
        self.entries.iter().map(|(k, v)| (k, Cow::Borrowed(v)))
    }
    fn entries_update(&mut self, lemma : &str, f : impl FnOnce(&mut Self::E)) {
        if let Some(e) = self.entries.get_mut(lemma) {
            f(e);
        } else {
            let mut e = ReDBEntries::new(self.db.clone(), lemma.to_string());
            f(&mut e);
            self.entries.insert(lemma.to_string(), e);
        }
    }
    fn synsets_get<'a>(&'a self, lexname : &str) -> Option<Cow<'a, Self::S>> {
        self.synsets.get(lexname).map(Cow::Borrowed)
    }
    fn synsets_insert(&mut self, lexname : String, synsets : BTSynsets) {
        for (id, synset) in synsets.into_iter() {
            if let Some(s) = self.synsets.get_mut(&lexname) {
                s.insert(id, synset);
            }
        }
    }
    fn synsets_iter<'a>(&'a self) -> impl Iterator<Item=(&'a String, Cow<'a, Self::S>)> {
        self.synsets.iter().map(|(k, v)| (k, Cow::Borrowed(v)))
    }
    fn synsets_update<X>(&mut self, lexname : &str, f : impl FnOnce(&mut Self::S) -> X) -> X {
        if let Some(s) = self.synsets.get_mut(lexname) {
            f(s)
        } else {
            let mut s = ReDBSynsets::new(self.db.clone(), lexname.to_string());
            let result = f(&mut s);
            self.synsets.insert(lexname.to_string(), s);
            result
        }
    }
    fn synsets_contains_key(&self, lexname : &str) -> bool {
        self.synsets_get(lexname).is_some()
    }
    fn synset_id_to_lexfile_get(&self, synset_id : &SynsetId) -> Option<&String> {
        panic!("TODO");
    }
    fn synset_id_to_lexfile_insert(&mut self, synset_id : SynsetId, lexfile : String) {
        panic!("TODO");
    }
    fn sense_links_to_get(&self, sense_id : &SenseId) -> Option<&Vec<(SenseRelType, SenseId)>> {
        panic!("TODO");
    }
    fn sense_links_to_get_or(&mut self, sense_id : SenseId, f : impl FnOnce() -> Vec<(SenseRelType, SenseId)>) 
        -> &mut Vec<(SenseRelType, SenseId)> {
            panic!("TODO");
    }
    fn sense_links_to_update(&mut self, sense_id : &SenseId, f : impl FnOnce(&mut Vec<(SenseRelType, SenseId)>)) {
        panic!("TODO");
    }
    fn sense_links_to_push(&mut self, sense_id : SenseId, rel : SenseRelType, target : SenseId) {
        panic!("TODO");
    }
    fn set_sense_links_to(&mut self, links_to : HashMap<SenseId, Vec<(SenseRelType, SenseId)>>) {
        panic!("TODO");
    }
    fn links_to_get(&self, synset_id : &SynsetId) -> Option<&Vec<(SynsetRelType, SynsetId)>> {
        panic!("TODO");
    }
    fn links_to_get_or(&mut self, synset_id : SynsetId, f : impl FnOnce() -> Vec<(SynsetRelType, SynsetId)>) 
        -> &mut Vec<(SynsetRelType, SynsetId)> {
            panic!("TODO");
    }
    fn links_to_update(&mut self, synset_id : &SynsetId, f : impl FnOnce(&mut Vec<(SynsetRelType, SynsetId)>)) {
        panic!("TODO");
    }
    fn links_to_push(&mut self, synset_id : SynsetId, rel : SynsetRelType, target : SynsetId) {
        panic!("TODO");
    }
    fn set_links_to(&mut self, links_to : HashMap<SynsetId, Vec<(SynsetRelType, SynsetId)>>) {
        panic!("TODO");
    }
    fn sense_id_to_lemma_pos_get(&self, sense_id : &SenseId) -> Option<&(String, PosKey)> {
        panic!("TODO");
    }
    fn sense_id_to_lemma_pos_insert(&mut self, sense_id : SenseId, lemma_pos : (String, PosKey)) {
        panic!("TODO");
    }
    fn deprecations_get(&self) -> &Vec<DeprecationRecord> {
        panic!("TODO");
    }
    fn deprecations_push(&mut self, record : DeprecationRecord) {
        panic!("TODO");
    }
}

#[derive(Clone)]
pub struct ReDBEntries {
    db: Rc<Database>,
    key : String
}

impl ReDBEntries {
    fn new(db : Rc<Database>, key : String) -> ReDBEntries {
        ReDBEntries { db, key }
    }
}

impl Entries for ReDBEntries {
    fn entry<'a>(&'a self, lemma : &str, pos_key : &PosKey) -> Option<Cow<'a, Entry>> {
        panic!("TODO");
    }
    fn insert_entry(&mut self, lemma : String, pos : PosKey, entry : Entry) {
        panic!("TODO");
    }
    fn update_entry<X>(&mut self, lemma : &str, pos_key : &PosKey,
        f : impl FnOnce(&mut Entry) -> X) -> Result<X> {
        panic!("TODO");
        }
    fn remove_entry(&mut self, lemma : &str, pos_key : &PosKey) -> Option<Entry> {
        panic!("TODO");
    }

    fn entry_by_lemma<'a>(&'a self, lemma : &str) -> Vec<Cow<'a, Entry>> {
        panic!("TODO");
    }
    fn entry_by_lemma_with_pos<'a>(&'a self, lemma : &str) -> Vec<(PosKey, Cow<'a, Entry>)> {
        panic!("TODO");
    }
    fn entry_by_lemma_ignore_case<'a>(&'a self, lemma : &str) -> Vec<Cow<'a, Entry>> {
        panic!("TODO");
    }

    //fn iter(&self) -> impl Iterator<Item=(&String,Vec<(&PosKey, &Entry)>)> {
    //    //SynsetIterator::new(self.db.clone())
    //    panic!("TODO")
    //}
    fn entries<'a>(&'a self) -> impl Iterator<Item=(String, PosKey, Cow<'a, Entry>)> {
;
        let txn = self.db.begin_read().unwrap();
        let table = txn.open_table(ENTRIES_TABLE).unwrap();
        EntryIterator::new(txn, table, |table| {
            table.iter().unwrap()
        }).map(|(l,p,e)| (l,p,Cow::Owned(e)))
    }

    fn into_entries(self) -> impl Iterator<Item=(String, PosKey, Entry)> {
        let txn = self.db.begin_read().unwrap();
        let table = txn.open_table(ENTRIES_TABLE).unwrap();
        EntryIterator::new(txn, table, |table| {
            table.iter().unwrap()
        })
    }

    fn n_entries(&self) -> usize {
        panic!("TODO");
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
}

impl Synsets for ReDBSynsets {
    fn get<'a>(&'a self, id : &SynsetId) -> Option<Cow<'a, Synset>> {
        panic!("TODO");
    }
    fn insert(&mut self, id : SynsetId, sysnet : Synset) -> Option<Synset> {
        panic!("TODO");
    }
    fn update<X>(&mut self, id : &SynsetId, f : impl FnOnce(&mut Synset) -> X) -> Result<X> {
        panic!("TODO");
    }
    fn iter<'a>(&'a self) -> impl Iterator<Item=(SynsetId, Cow<'a, Synset>)> + 'a {
        let txn = self.db.begin_read().unwrap();
        let table = txn.open_table(SYNSETS_TABLE).unwrap();
        SynsetIterator::new(txn, table, |table| {
            table.iter().unwrap()
        }).map(|(k, v)| (k, Cow::Owned(v)))
    }
    fn into_iter(self) -> impl Iterator<Item=(SynsetId, Synset)> {
        let txn = self.db.begin_read().unwrap();
        let table = txn.open_table(SYNSETS_TABLE).unwrap();
        SynsetIterator::new(txn, table, |table| {
            table.iter().unwrap()
        })
    }
    fn len(&self) -> usize {
        panic!("TODO");
    }
    fn remove_entry(&mut self, id : &SynsetId) -> Option<(SynsetId, Synset)> {
        panic!("TODO");
    }
}

#[self_referencing]
pub struct SynsetIterator {
    txn: ReadTransaction,
    table: ReadOnlyTable<String, String>,
    #[borrows(table)]
    #[covariant]
    inner: Range<'this, String, String>
}

impl Iterator for SynsetIterator {
    type Item = (SynsetId, Synset);

    fn next(&mut self) -> Option<Self::Item> {
        self.with_inner_mut(|inner| {
            inner.next().map(|res| {
                let (k, v) = res.unwrap();
                (SynsetId::new_owned(k.value()), deserialize_synset(v.value()))
            })
        })
    }
}

fn deserialize_synset(s : String) -> Synset {
    panic!("TODO")
}


#[self_referencing]
pub struct EntryIterator {
    txn: ReadTransaction,
    table: ReadOnlyTable<(String, String), String>,
    #[borrows(table)]
    #[covariant]
    inner: Range<'this, (String, String), String>
}

impl Iterator for EntryIterator {
    type Item = (String, PosKey, Entry);

    fn next(&mut self) -> Option<Self::Item> {
        self.with_inner_mut(|inner| {
            inner.next().map(|res| {
                let (ks, v) = res.unwrap();
                let (k1, k2) = ks.value();
                (k1,
                    PosKey::new(k2),
                    deserialize_entry(v.value()))
            })
        })
    }
}

fn deserialize_entry(s : String) -> Entry {
    panic!("TODO")
}
