---
title: "EWE User Study \u2014 Task Set 1"
---

# Task Set 1

Moderator/reference copy -- includes issue number, URL, and the gold-fix PR link. See `task_sets.csv` for the same metadata in tabular form.

## Task 1 — 22-karat gold

**Issue:** [#1182](https://github.com/globalwordnet/english-wordnet/issues/1182)  \
**Category:** definition  \
**Size:** tiny  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1187](https://github.com/globalwordnet/english-wordnet/pull/1187)

**Synset**
 oewn-14663546-n (Interlingual Index: i113940)
(n) 22-karat gold an alloy that contains 87 per cent gold

**New definition**
an alloy that contains 91.67 per cent gold

**Motivation**
Hey, the 22 Karat Gold definition on OEWN is wrong. It says that it is 87 percent purity but it's really 91.67 percent purity. 

Please fix this for 2025.

\newpage

## Task 2 — 'forsooth' presents two definitions

**Issue:** [#1276](https://github.com/globalwordnet/english-wordnet/issues/1276)  \
**Category:** definition  \
**Size:** small  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1292](https://github.com/globalwordnet/english-wordnet/pull/1292)

**Synset**
ewn-00038658-r
forsooth

**New definition**
Please state the new definition you are proposing
- used to express disbelief

**Motivation**
The original definition:
- an archaic word originally meaning ‘in truth’ but now usually used to express
    disbelief
presents 2 senses (in truth, disbelief), it should be split in the synset of indeed
00037864-r

\newpage

## Task 3 — Urubupunga

**Issue:** [#1136](https://github.com/globalwordnet/english-wordnet/issues/1136)  \
**Category:** definition  \
**Size:** medium  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1161](https://github.com/globalwordnet/english-wordnet/pull/1161)

**Synset**
oewn-09490895-n (Interlingual Index: i86375)
(n) Urubupunga, [Urubupunga Falls](https://en-word.net/lemma/Urubupunga%20Falls) a waterfall in the Parana river in Brazil

oewn-09453320-n (Interlingual Index: i86159)
(n) [Guaira](https://en-word.net/lemma/Guaira), [Guaira Falls](https://en-word.net/lemma/Guaira%20Falls), [Sete Quedas](https://en-word.net/lemma/Sete%20Quedas) a great waterfall on the border between Brazil and Paraguay

**New definition**
a former waterfall on the Parana river in Brazil; now impounded by the Engineer Souza Dias Dam and the Ilha Solteira Dam

a former waterfall on the border between Brazil and Paraguay; now impounded by the Itaipu Dam

**Motivation**
This took me a long time to discover what 'Urubupunga' refers to. Googling only seems to show a bus-stop in Sao Paulo. 

I eventually found this source:
https://www.google.ie/books/edition/Survey_of_the_Brazilian_Economy/0gQpMyluTaMC?hl=en&gbpv=1&dq=Urubupunga&pg=RA1-PA170&printsec=frontcover

Which says:

> CELUSA (Centrais Eletricas de Urubupungá) created in 1961 to harness the Urubupungá falls through the construction of two large plants: Jupiá [... and...] Ilha Solteira

[Jupiá was since renamed to Engineer Souza Dias](https://en.wikipedia.org/wiki/Engineer_Souza_Dias_Dam)

The second case is [described in Wikipedia](https://en.wikipedia.org/wiki/Gua%C3%ADra_Falls)

We should also add accents to the relevant terms.

\newpage

## Task 4 — 'hayfield'

**Issue:** [#1387](https://github.com/globalwordnet/english-wordnet/issues/1387)  \
**Category:** duplicate_or_merge  \
**Size:** medium  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1395](https://github.com/globalwordnet/english-wordnet/pull/1395)

**Lemmas**
'hayfield', 'hay field'

**Definition**
A field where grass grows for hay.

**Relation**
hypernym -> `08596880-n`

**Significance**
Currently, 'hayfield' is covered in the (only) synset for 'meadow'. 

```
08588163-n: hayfield, meadow
    a piece of land covered or mostly covered with grass; a field where grass or alfalfa are grown to be made into hay
    hypernym: 08615857-n (grassland)
```

This definition and synset merge two obviously quite distinct concepts. I think it is better to keep 'meadow' in the existing synset and shorten its definition to 'a piece of land covered with grass and wild plants'. A new synset will be created for 'hay field'

\newpage

## Task 5 — 'ignore'

**Issue:** [#1060](https://github.com/globalwordnet/english-wordnet/issues/1060)  \
**Category:** deprecation  \
**Size:** medium  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1087](https://github.com/globalwordnet/english-wordnet/pull/1087)

**Synset**
oewn-00596900-v (Interlingual Index: i24739)
(v) ignore 
> be ignorant of or in the dark about

**Motivation**
This sense does not fit with my understanding of English. "I am ignorant of something" does not mean "I ignore something".

It has no occurrences in any of the UFSAC corpora, suggesting this sense is not used.

I suggest deprecating to the adjective 'ignorant'

\newpage

## Task 6 — Quotation with apostrophe

**Issue:** [#1340](https://github.com/globalwordnet/english-wordnet/issues/1340)  \
**Category:** other  \
**Size:** tiny  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1357](https://github.com/globalwordnet/english-wordnet/pull/1357)

src/yaml/adv.all.yaml: 

` - \'The Pentagon\' is often used metonymically to refer to the Department of Defense`

does not align with quotation coding currently used in OEWN

Quoting with apostrophes has been superseded with proper quotes for some time.
I know, this is hardly visible and hard to type but it has the advantage of 
-not colliding/multiplexing with anything 
-respecting the semantics of the apostrophe character, 
-making it easy to search for and process quotations (because it uses distinct dedicated --opening, closing-- characters)

```
 ‘ 2018 single quotation, left
 ’ 2019 single quotation, right
 ‘quoted’

```
How to type them in Linux if noot copying from a charmap
Ctrl + Shift + U, then 2018, then Enter → ‘
Ctrl + Shift + U, then 2019, then Enter → ’

This should probably be checked (and transformed if neeed)  by EWE when introducing new stuff and/or be added to format requirements.

\newpage

## Task 7 — 'stinks', 'sucks'

**Issue:** [#1041](https://github.com/globalwordnet/english-wordnet/issues/1041)  \
**Category:** other  \
**Size:** medium  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1066](https://github.com/globalwordnet/english-wordnet/pull/1066)

**Synsets**
oewn-02680129-v (Interlingual Index: i35032)
(v) suck, [blow](https://en-word.net/lemma/blow) be inadequate or objectionable “this sucks!” “this blows!”

oewn-02679962-v (Interlingual Index: i35031)
(v) stink be extremely bad in quality or in one's performance “This term paper stinks!”

**Motivation**
I think these two senses are fully substitutable (if somethings "suck" then it "stinks" and vica versa)

\newpage

## Task 8 — Spurious relation

**Issue:** [#1228](https://github.com/globalwordnet/english-wordnet/issues/1228)  \
**Category:** relation  \
**Size:** tiny  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1229](https://github.com/globalwordnet/english-wordnet/pull/1229)

**Existing relation**
oewn-10807146-n -> oewn-09654651-n

**Proposed change**
Remove it

**Motivation**
This slipped in as a bug from trying to remove some anti-feminist slurs

\newpage

## Task 9 — 'eitchen midden'

**Issue:** [#1388](https://github.com/globalwordnet/english-wordnet/issues/1388)  \
**Category:** relation  \
**Size:** small  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1396](https://github.com/globalwordnet/english-wordnet/pull/1396)

**Synset**
```
08578097-n: eitchen midden, midden, kitchen midden
    (archeology) a mound of domestic refuse containing shells and animal bones marking the site of a prehistoric settlement
    hypernym: 08577564-n (dump, garbage dump, trash dump, rubbish dump, wasteyard, waste-yard, dumpsite)
    domain_topic: 06153532-n (archeology, archaeology)
```

**Lemma**
'eitchen midden'

**Motivation**
Obviously a typo, the correct lemma has been added as well.

\newpage

## Task 10 — 'positive', 'comparative', 'superlative'

**Issue:** [#1380](https://github.com/globalwordnet/english-wordnet/issues/1380)  \
**Category:** relation  \
**Size:** medium  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1393](https://github.com/globalwordnet/english-wordnet/pull/1393)

**Lemmas**
'positive'
'comparative'
'superlative'

**Definition**
of an adjective in the primary form, not the comparative or superlative form
of an adjective in the form showing comparison
of an adjective in the form showing greatest degree

**Relation**
pertainym => 06333461-n
pertainym => 06333686-n
pertainym => 06334277-n

**Significance**
We have nominal senses for 'positive', 'comparative', 'superlative' but not the adjectival sense

\newpage
