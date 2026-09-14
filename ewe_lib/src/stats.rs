use crate::wordnet::{Lexicon, Result, SynsetId};
use std::collections::{HashMap, HashSet};

/// Precomputed hypernym-hierarchy data shared by the test-pattern checks below (based on the
/// "test patterns" in Lohk, Fellbaum & Võhandu, "Tuning Hierarchies in Princeton WordNet", GWC
/// 2016), so `ewe stats` only walks the wordnet and computes the transitive hypernym closure
/// once even when several checks are requested together.
pub struct HypernymGraph {
    /// Direct `hypernym`/`instance_hypernym` parents of each synset.
    direct_parents: HashMap<SynsetId, HashSet<SynsetId>>,
    /// Direct hyponyms of each synset - the inverse of `direct_parents`.
    direct_children: HashMap<SynsetId, HashSet<SynsetId>>,
    /// The full set of transitive hypernym ancestors of each synset (excludes the synset
    /// itself, even where a genuine cycle exists - that's reported separately by
    /// `validate::check_no_loops`'s `Loop` error).
    ancestors: HashMap<SynsetId, HashSet<SynsetId>>,
    members: HashMap<SynsetId, Vec<String>>,
}

impl HypernymGraph {
    /// Builds the graph via the same semi-naive fixed-point iteration
    /// `validate::check_no_loops` uses to compute the transitive hypernym closure - hypernym
    /// hierarchies are shallow in practice, so this converges in a handful of passes.
    pub fn build<L: Lexicon>(wn: &L) -> Result<HypernymGraph> {
        let mut direct_parents: HashMap<SynsetId, HashSet<SynsetId>> = HashMap::new();
        let mut direct_children: HashMap<SynsetId, HashSet<SynsetId>> = HashMap::new();
        let mut members: HashMap<SynsetId, Vec<String>> = HashMap::new();
        for s in wn.synsets()? {
            let (synset_id, synset) = s?;
            let mut parents = HashSet::new();
            parents.extend(synset.hypernym.iter().cloned());
            parents.extend(synset.instance_hypernym.iter().cloned());
            for parent in &parents {
                direct_children
                    .entry(parent.clone())
                    .or_default()
                    .insert(synset_id.clone());
            }
            direct_parents.insert(synset_id.clone(), parents);
            members.insert(synset_id, synset.members.clone());
        }

        let ids: Vec<SynsetId> = direct_parents.keys().cloned().collect();
        let mut ancestors: HashMap<SynsetId, HashSet<SynsetId>> = direct_parents.clone();
        let mut changed = true;
        while changed {
            changed = false;
            for id in &ids {
                let current: Vec<SynsetId> = ancestors[id].iter().cloned().collect();
                let extension: Vec<SynsetId> = current
                    .iter()
                    .flat_map(|p| ancestors.get(p).into_iter().flatten().cloned())
                    .collect();
                let entry = ancestors.get_mut(id).expect("id came from this map's own keys");
                let before = entry.len();
                entry.extend(extension);
                if entry.len() != before {
                    changed = true;
                }
            }
        }

        Ok(HypernymGraph {
            direct_parents,
            direct_children,
            ancestors,
            members,
        })
    }

    fn is_ancestor(&self, synset: &SynsetId, candidate: &SynsetId) -> bool {
        self.ancestors
            .get(synset)
            .is_some_and(|a| a.contains(candidate))
    }
}

/// One occurrence of a word that is a member of some synset and is *also* a member of one of
/// that synset's transitive hypernym ancestors - i.e. the same word shows up twice while
/// walking up the hypernym tree. This is almost always a modelling mistake (two senses that
/// should be merged, or a hypernym link that was pointed at the wrong synset) rather than a
/// legitimate case of a word being hyponymous with itself.
#[derive(Debug, Clone)]
pub struct SelfHypernymyInstance {
    pub word: String,
    /// The synset `word` is a direct member of.
    pub synset: SynsetId,
    /// A hypernym ancestor of `synset` that `word` is also a member of.
    pub ancestor: SynsetId,
}

/// Finds every [`SelfHypernymyInstance`] in `graph`.
pub fn find_self_hypernymy(graph: &HypernymGraph) -> Vec<SelfHypernymyInstance> {
    let mut instances = Vec::new();
    for (synset_id, own_members) in graph.members.iter() {
        let Some(anc) = graph.ancestors.get(synset_id) else { continue };
        for word in own_members {
            for ancestor_id in anc {
                if ancestor_id == synset_id {
                    continue;
                }
                if graph
                    .members
                    .get(ancestor_id)
                    .is_some_and(|m| m.iter().any(|w| w == word))
                {
                    instances.push(SelfHypernymyInstance {
                        word: word.clone(),
                        synset: synset_id.clone(),
                        ancestor: ancestor_id.clone(),
                    });
                    break;
                }
            }
        }
    }
    instances.sort_by(|a, b| {
        (a.word.as_str(), a.synset.as_str()).cmp(&(b.word.as_str(), b.synset.as_str()))
    });
    instances
}

/// A synset with a direct hypernym edge that's redundant: the target is also reachable via one
/// of the synset's *other* direct hypernym parents, so removing the direct edge wouldn't change
/// the synset's transitive hypernym closure at all.
#[derive(Debug, Clone)]
pub struct ShortcutInstance {
    pub synset: SynsetId,
    /// The hypernym that's also reachable indirectly (the dotted line in Lohk et al.'s Fig. 1).
    pub redundant_hypernym: SynsetId,
    /// The other direct hypernym parent through which `redundant_hypernym` is still reachable.
    pub via: SynsetId,
}

/// Finds "shortcut" instances (Lohk et al., 2016, §4.1). Requires a synset to have at least two
/// direct hypernym parents, since a shortcut is only redundant relative to some other path.
pub fn find_shortcuts(graph: &HypernymGraph) -> Vec<ShortcutInstance> {
    let mut instances = Vec::new();
    for (synset_id, parents) in graph.direct_parents.iter() {
        if parents.len() < 2 {
            continue;
        }
        for target in parents {
            for via in parents {
                if via == target {
                    continue;
                }
                if graph.is_ancestor(via, target) {
                    instances.push(ShortcutInstance {
                        synset: synset_id.clone(),
                        redundant_hypernym: target.clone(),
                        via: via.clone(),
                    });
                    break;
                }
            }
        }
    }
    instances.sort_by(|a, b| {
        (a.synset.as_str(), a.redundant_hypernym.as_str())
            .cmp(&(b.synset.as_str(), b.redundant_hypernym.as_str()))
    });
    instances
}

/// A set of synsets that all share the same pair of direct hypernym parents - a complete
/// bipartite subgraph between `parents` and `children`.
#[derive(Debug, Clone)]
pub struct DenseComponentInstance {
    pub parents: (SynsetId, SynsetId),
    pub children: Vec<SynsetId>,
}

/// Finds "dense component" instances (Lohk et al., 2016, §4.4): synsets connected to the same
/// two (or more) hypernym parents via multiple inheritance, forming an unusually dense patch of
/// the hierarchy. A synset with more than two shared parents is reported once per pair of those
/// parents, matching how the paper's own example (Fig. 4) has overlapping rather than merged
/// instances.
pub fn find_dense_components(graph: &HypernymGraph) -> Vec<DenseComponentInstance> {
    let mut by_parent_pair: HashMap<(SynsetId, SynsetId), Vec<SynsetId>> = HashMap::new();
    for (synset_id, parents) in graph.direct_parents.iter() {
        if parents.len() < 2 {
            continue;
        }
        let mut sorted_parents: Vec<&SynsetId> = parents.iter().collect();
        sorted_parents.sort();
        for i in 0..sorted_parents.len() {
            for j in (i + 1)..sorted_parents.len() {
                let key = (sorted_parents[i].clone(), sorted_parents[j].clone());
                by_parent_pair.entry(key).or_default().push(synset_id.clone());
            }
        }
    }
    let mut instances: Vec<DenseComponentInstance> = by_parent_pair
        .into_iter()
        .filter(|(_, children)| children.len() >= 2)
        .map(|(parents, mut children)| {
            children.sort();
            DenseComponentInstance { parents, children }
        })
        .collect();
    instances.sort_by(|a, b| a.parents.cmp(&b.parents));
    instances
}

/// A hypernym `parent` whose member word `word` is also a (proper) suffix of a member of at
/// least two of its direct hyponyms, where at least one of those hyponyms additionally has
/// another, unrelated hypernym parent.
#[derive(Debug, Clone)]
pub struct CompoundPatternInstance {
    pub parent: SynsetId,
    pub word: String,
    /// Direct hyponyms of `parent` whose member compounds `word` (e.g. {baseball} for {ball}).
    pub children: Vec<SynsetId>,
    /// The subset of `children` that also has a second, unrelated hypernym parent.
    pub children_with_extra_hypernym: Vec<SynsetId>,
}

/// Finds "compound" pattern instances (Lohk et al., 2016, §4.3) - the one test pattern that
/// looks at synset *content* rather than pure graph shape. Uses the same "proper suffix" rule
/// (Nadig et al., 2008) the paper's own {ball}/{baseball}/... example relies on: a subordinate
/// whose member merely compounds the superordinate's member word is a candidate for having been
/// mis-modelled as a hypernym relation rather than a lexical/morphological one, and the
/// "extra superordinate" condition (at least one such subordinate has another, unrelated
/// hypernym) is what the paper uses to separate real cases from ordinary IS-A hierarchies with
/// compound lemmas (e.g. {basketball} genuinely is a {ball}, but is also {basketball
/// equipment}).
pub fn find_compound_pattern(graph: &HypernymGraph) -> Vec<CompoundPatternInstance> {
    let mut instances = Vec::new();
    for (parent_id, words) in graph.members.iter() {
        let Some(children) = graph.direct_children.get(parent_id) else { continue };
        if children.len() < 2 {
            continue;
        }
        for word in words {
            let word_lower = word.to_lowercase();
            let mut matching_children: Vec<SynsetId> = children
                .iter()
                .filter(|child| {
                    graph.members.get(*child).is_some_and(|child_members| {
                        child_members.iter().any(|m| {
                            let m_lower = m.to_lowercase();
                            m_lower.len() > word_lower.len() && m_lower.ends_with(&word_lower)
                        })
                    })
                })
                .cloned()
                .collect();
            if matching_children.len() < 2 {
                continue;
            }
            let children_with_extra_hypernym: Vec<SynsetId> = matching_children
                .iter()
                .filter(|c| graph.direct_parents.get(*c).is_some_and(|p| p.len() > 1))
                .cloned()
                .collect();
            if children_with_extra_hypernym.is_empty() {
                continue;
            }
            matching_children.sort();
            instances.push(CompoundPatternInstance {
                parent: parent_id.clone(),
                word: word.clone(),
                children: matching_children,
                children_with_extra_hypernym,
            });
        }
    }
    instances.sort_by(|a, b| {
        (a.parent.as_str(), a.word.as_str()).cmp(&(b.parent.as_str(), b.word.as_str()))
    });
    instances
}
