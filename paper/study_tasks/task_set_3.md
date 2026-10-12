---
title: "EWE User Study \u2014 Task Set 3"
---

# Task Set 3

Moderator/reference copy -- includes issue number, URL, and the gold-fix PR link. See `task_sets.csv` for the same metadata in tabular form.

## Task 1 — nonprofessional

**Issue:** [#1003](https://github.com/globalwordnet/english-wordnet/issues/1003)  \
**Category:** definition  \
**Size:** tiny  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1009](https://github.com/globalwordnet/english-wordnet/pull/1009)

**Synset**

oewn-01875320-a (Interlingual Index: i10263)

(a) nonprofessional not professional; not engaged in a profession or engaging in as a profession or for gain “the nonprofessional wives of his male colleagues” “nonprofessional actors”

**New definition**

not professional; not engaged in a profession or engaging in nor for gain

**Motivation**

The original description is confusing, as far as I understood, it is wrong.

\newpage

## Task 2 — Add `green` to https://en-word.net/id/oewn-90016161-s

**Issue:** [#1015](https://github.com/globalwordnet/english-wordnet/issues/1015)  \
**Category:** definition  \
**Size:** small  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1048](https://github.com/globalwordnet/english-wordnet/pull/1048)

**Affected synsets**
https://en-word.net/id/oewn-90016161-s

**Proposed changes**
Add `green` as a member of the mentioned synset

**Motivation**
See definition number 9 at: https://en.wiktionary.org/wiki/green#Adjective

\newpage

## Task 3 — 'drive in'

**Issue:** [#1036](https://github.com/globalwordnet/english-wordnet/issues/1036)  \
**Category:** definition  \
**Size:** medium  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1063](https://github.com/globalwordnet/english-wordnet/pull/1063)

**Synset**
oewn-01116045-v (Interlingual Index: i27165)
(v) drive in (([ball](https://en-word.net/id/oewn-00472688-n))) cause a run or runner to be scored “His line double drove in Jim Lemon with the winning run”

**New definition**
score a run or cause a run to be scored

**Motivation**
Corpus usage supports the primary active use of this term. Add `vtai` frame here.

\newpage

## Task 4 — pack rat/packrat

**Issue:** [#1143](https://github.com/globalwordnet/english-wordnet/issues/1143)  \
**Category:** duplicate_or_merge  \
**Size:** small  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1152](https://github.com/globalwordnet/english-wordnet/pull/1152)

**Synsets**
oewn-10410299-n (Interlingual Index: i91815)
(n) packrat a collector of miscellaneous useless objects

oewn-10300973-n (Interlingual Index: i91146)
(n) magpie, pack rat, scavenger someone who collects things that have been discarded by others

**Motivation**
Wiktionary has: One who collects or hoards, especially unnecessary things.
I think we can merge 10410299-n into 10300973-n, give it two variants _pack rat, packrat_ and maybe define as "someone who collects things, especially useless objects or those that have been discarded by others"

Or if we think scavenger is different enough, then keep both, and move magpie and pack rat to 10410299-n, ...

\newpage

## Task 5 — Create adjective entry for "fun"

**Issue:** [#1172](https://github.com/globalwordnet/english-wordnet/issues/1172)  \
**Category:** new_lemma_or_sense  \
**Size:** small  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1325](https://github.com/globalwordnet/english-wordnet/pull/1325)

I don't have enough experience in WN yet to suggest which synset this should belong to.

The lemma "fun" exists in WN as a partOfSpeech noun but the adjective form does not exist.

I discovered this randomly while exploring the WN data file.

The new entry would end up looking something like:

```
   <LexicalEntry id="oewn-fun-a">
      <Lemma writtenForm="fun" partOfSpeech="a">
        <Pronunciation>f[U+028C]n</Pronunciation>
      </Lemma>

      <Sense id="..." synset="..."/>

    </LexicalEntry>
```


- Rob

\newpage

## Task 6 — 'deportation'

**Issue:** [#1207](https://github.com/globalwordnet/english-wordnet/issues/1207)  \
**Category:** other  \
**Size:** small  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1329](https://github.com/globalwordnet/english-wordnet/pull/1329)

**Synsets**
oewn-00208610-n (Interlingual Index: i36497)
(n) deportation, [exile](https://en-word.net/lemma/exile), [expatriation](https://en-word.net/lemma/expatriation), [transportation](https://en-word.net/lemma/transportation) the act of expelling a person from their native land “men in exile dream of hope” “his deportation to a penal colony” “the expatriation of wealthy farmers” “the sentence was one of transportation for life”

oewn-00208155-n (Interlingual Index: i36494)
(n) deportation the expulsion from a country of an undesirable alien

**Motivation**
These two senses are not really distinguished, the "act of expelling" and "explusion" are not really different

\newpage

## Task 7 — unintentional

**Issue:** [#1281](https://github.com/globalwordnet/english-wordnet/issues/1281)  \
**Category:** other  \
**Size:** medium  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1282](https://github.com/globalwordnet/english-wordnet/pull/1282)

designed, intentional, unintentional: done or made or performed with purpose and intent
I think the word unintentional is incorrect in this context as it is not corresponding with designed and intentional

\newpage

## Task 8 — Male homophobic slurs

**Issue:** [#1100](https://github.com/globalwordnet/english-wordnet/issues/1100)  \
**Category:** relation  \
**Size:** tiny  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1159](https://github.com/globalwordnet/english-wordnet/pull/1159)

**Existing relation**
oewn-10095821-n (Interlingual Index: i89870)
(n) [pouf](https://en-word.net/lemma/pouf), [fag](https://en-word.net/lemma/fag), faggot, [fagot](https://en-word.net/lemma/fagot), [queen](https://en-word.net/lemma/queen), [pansy](https://en-word.net/lemma/pansy), [poof](https://en-word.net/lemma/poof), [poove](https://en-word.net/lemma/poove), [fairy](https://en-word.net/lemma/fairy), [queer](https://en-word.net/lemma/queer), [nance](https://en-word.net/lemma/nance) (([sex](https://en-word.net/id/oewn-00845915-n))) (([LGBT](https://en-word.net/id/oewn-84123305-n))) offensive term for a homosexual man

to

oewn-10202544-n (Interlingual Index: i90552)
(n) [gay](https://en-word.net/lemma/gay), [homo](https://en-word.net/lemma/homo), [homophile](https://en-word.net/lemma/homophile), [homosexual](https://en-word.net/lemma/homosexual) someone who is sexually attracted to persons of the same sex

**Proposed change**
Change to 
oewn-83837368-n (Interlingual Index: in)
(n) gay man, [shirtlifter](https://en-word.net/lemma/shirtlifter) a homosexual man

And move 'shirtlifter' to this synset

**Motivation**
Captures these as being terms that refer to 'men'. This follows the same structure used for slurs for gay women.

\newpage

## Task 9 — 'carnivore' -> 'carnivoran'

**Issue:** [#1378](https://github.com/globalwordnet/english-wordnet/issues/1378)  \
**Category:** relation  \
**Size:** small  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1392](https://github.com/globalwordnet/english-wordnet/pull/1392)

**Synset**
```
02077948-n: carnivore
    a terrestrial or aquatic flesh-eating mammal
    (terrestrial carnivores have four or five clawed digits on each limb)
    hypernym: 01889397-n (placental, placental mammal, eutherian, eutherian mammal)
    hyponym: 02084842-n (fissiped mammal, fissiped); 02085998-n (canine, canid); 02123649-n (feline, felid, cat); 02134305-n (bear); 02137623-n (viverrine, viverrine mammal); 02443957-n (musteline mammal, mustelid, musteline); 02510280-n (procyonid)

01327072-n: carnivore
    any animal that feeds on flesh
    (Tyrannosaurus Rex was a large carnivore; insectivorous plants are considered carnivores)
    hypernym: 02155392-n (predator, predatory animal)
```

**Lemma**
Remove 'carnivore' from `02077948-n` and replace with 'carnivoran', 'carnivorous mammal'

**Motivation**
Not all 'carnivores' are mammals as suggested by this synset.

Source: https://en.wikipedia.org/wiki/Carnivora

\newpage

## Task 10 — 'suggest'/'hint'

**Issue:** [#1364](https://github.com/globalwordnet/english-wordnet/issues/1364)  \
**Category:** relation  \
**Size:** medium  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1366](https://github.com/globalwordnet/english-wordnet/pull/1366)

**Existing relation**
```
00929401-v: hint, suggest
    drop a hint; intimate by a hint
    hypernym: 00930591-v (convey)
    hyponym: 00929682-v (intimate, adumbrate, insinuate); 00929871-v (clue in); 01028737-v (allude, touch, advert)

00932768-v: suggest, intimate
    imply as a possibility
    (The evidence suggests a need for more clarification)
    hypernym: 00932561-v (imply)
    hyponym: 00933047-v (make out)
```

**Proposed change**
Make 'hint' a specific kind of suggesting. This would change the hypernym of `00929401-v` to `00932768-v` and remove 'suggest' from `00929401-v`

**Motivation**
The senses of suggest are too similar, and this creates a sense distinction not found in other dictionaries:

https://www.dictionary.com/browse/suggest
https://www.merriam-webster.com/dictionary/suggest
https://en.wiktionary.org/wiki/suggest

The basic problem is that hinting is a specific way of suggesting.

\newpage
