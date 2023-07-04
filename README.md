English WordNet Editor (EWE)
============================

English WordNet Editor is an editor for working with Open English WordNet, which
is developed at https://github.com/globalwordnet/english-wordnet. This ensures
that changes are consistent and validates the resulting files.

Installation
------------

Release builds can be obtained from the [release section](https://github.com/jmccrae/ewe/releases). These are executables and can be run directly. 
We recommend saving these to the same folder that contains the Git repository
for Open English WordNet. EWE can be started by executing this file

Usage
-----

EWE is menu-driven, please choose the appropriate option when it has started
you should see something like this:

```

         ,ww                             
   wWWWWWWW_)  Welcome to EWE            
   `WWWWWW'    - English WordNet Editor  
    II  II                               

Loading WordNet
████████████████████████████████████████████████████████████████████████ 73/73

Please choose an option:
1. Add/delete/move entry
2. Add/delete a synset
3. Change a definition
4. Change an example
5. Change a relation
6. Validate
X. Exit EWE
Option> 
```

Building EWE
------------

EWE can be built with Cargo as follows

    cargo build --release

Automating EWE
--------------

EWE can be automated with an automaton file as follows

    ewe automaton.yaml /path/to/wn

An example of the usage of the automaton file is given below

```yaml
---
- add_entry:
    synset: 00001740-n
    lemma: bar
    pos: n
- delete_entry:
    synset: 00001740-n
    lemma: bar
- move_entry:
    synset: 00001740-n
    lemma: bar
    target_synset: 00001741-n
- add_synset:
    definition: something or someone
    lexfile: noun.animal
    pos: n
    lemmas:
      - bar
- delete_synset:
    synset: 00001740-n
    reason: "Duplicate (#123)"
    superseded_by: 00001741-n
- change_definition:
    synset: 00001740-n
    definition: This is a definition
- add_example:
    synset: 00001740-n
    example: This is an example
    source: This is a source
- delete_example:
    synset: 00001740-n
    number: 1
- add_relation:
    source: 00001740-n
    relation: hypernym
    target: 00001741-n
- delete_relation:
    source: 00001740-n
    source_sense: "example%1:09:00::"
    target: 00001741-n
    target_sense: "target%1:10:00::'"
- reverse_relation:
    source: 00001740-n
    target: 00001741-n
- validate
```
