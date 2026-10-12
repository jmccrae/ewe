---
title: "EWE User Study \u2014 Task Set 2"
---

# Task Set 2

Moderator/reference copy -- includes issue number, URL, and the gold-fix PR link. See `task_sets.csv` for the same metadata in tabular form.

## Task 1 — 'observe' ungrammatical definition

**Issue:** [#1055](https://github.com/globalwordnet/english-wordnet/issues/1055)  \
**Category:** definition  \
**Size:** tiny  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1156](https://github.com/globalwordnet/english-wordnet/pull/1156)

**Synset**
oewn-02584595-v (Interlingual Index: i34578)
(v) [keep](https://en-word.net/lemma/keep), observe, [celebrate](https://en-word.net/lemma/celebrate) behave as expected during of holidays or rites “Keep the commandments” “celebrate Christmas” “Observe Yom Kippur”

**New definition**
"behave as expected during holidays or rites"

**Motivation**
Definition is not grammatical

\newpage

## Task 2 — Some very poor definitions of species

**Issue:** [#1225](https://github.com/globalwordnet/english-wordnet/issues/1225)  \
**Category:** definition  \
**Size:** small  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1332](https://github.com/globalwordnet/english-wordnet/pull/1332)

**Synset**
 
* Chrysanthemum morifolium	`11981569-n` - 'of China'
* Thomomys bottae	`02357273-n` - 'of valleys and mountain meadows of western United States'
* Holothuria edulis	`02324699-n` - 'of warm coasts from Australia to Asia; used as food especially by Chinese' 
* Sylvilagus floridanus	`02328374-n` - 'widely distributed in United States except northwest and far west regions'
* Parahyaena brunnea	`02120298-n` - 'of southern Africa'

**New definition**

* A perennial flowering plant native to China
* A species of pocket gopher found in valleys and mountain meadows of the western United States
* A species of sea cucumber inhabiting warm coastal waters from Australia to Asia
* one of the most common North American rabbit species, widely distributed across the United States except in the northwest and far west region
* a carnivorous mammal native to southern Africa, noted for its shaggy dark coat and scavenging habits

**Motivation**
Definitions are not clear

\newpage

## Task 3 — Short Female Definitions

**Issue:** [#1071](https://github.com/globalwordnet/english-wordnet/issues/1071)  \
**Category:** definition  \
**Size:** medium  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1089](https://github.com/globalwordnet/english-wordnet/pull/1089)

Some female definitions (highlighted by @KexionGAO42 and team) are much shorter for the female equivalent than the male. We should make these definitions a consistent and fair length. I have edited from her original list [here](https://github.com/KexinGAO42/gender_bias_lexical_resource/blob/main/appendix/definitional_bias.txt) with changes that are already implemented in #883 (as the original analysis was on PWN3.0) and I have added suggested new definitions

```
aviator:N, someone who operates an aircraft
airwoman:F, a woman aviator => a female who operates an aircraft
airman:M, a male who operates an aircraft

assemblyman:M, someone who is a member of a legislative assembly => a man who is a member of a legislative assembly
assemblywoman:F, a woman assemblyman =>  a woman who is a member of a legislative assembly

horseman:M, a man skilled in equitation
horsewoman:F, a woman horseman => a woman skilled in equitation

oarsman:M, a male who rows a boat
oarswoman:F, a woman oarsman = > a female who rows a boat

policeman:M, a male member of a police force
policewoman:F, a woman policeman => a female member of a police force

scotchman:M, a male native or inhabitant of Scotland
scotchwoman:F, a woman who is a Scot => a female native or inhabitant of Scotland

statesman:M, a man who is a respected leader in national or international affairs
stateswoman:F, a woman statesman => a woman who is a respected leader in national or international affairs

businessman:M, a person engaged in commercial or industrial business (especially an owner or executive) => a man engaged in commercial or industrial business
businesswoman:F, a female businessperson => a woman engaged in commercial or industrial business
businessperson:N, a capitalist who engages in industrial commercial enterprise

newsman:M, a male person who investigates and reports or edits news stories
newswoman:F, a female newsperson => a female person who investigates and reports or edits news stories
newsperson:N, a person who investigates and reports or edits news stories
```

\newpage

## Task 4 — 'classic'

**Issue:** [#1263](https://github.com/globalwordnet/english-wordnet/issues/1263)  \
**Category:** duplicate_or_merge  \
**Size:** medium  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1334](https://github.com/globalwordnet/english-wordnet/pull/1334)

**Synset**
oewn-02479168-s (Interlingual Index: in)
(s) classic 
> of a well-known type; remarkably typical “the classic struggle between good and evil” “I woke up with all the classic symptoms of the flu” “she made the classic mistake of choosing style over substance”

oewn-02352097-s (Interlingual Index: in)
(s) classic 
> considered of the highest quality and lasting significance or worth “a classic car” “‘War and Peace’ is a classic novel”

oewn-00613690-s (Interlingual Index: in)
(s) classic, [classical](https://en-word.net/lemma/classical) 
> well-known and long-established in form or style “a classical ballet” “classic double-breasted suit” “the classic struggle between good and evil”

**Motivation**
These synsets are not clearly distinguished and in at least one case have the exact same example. 

I would suggest to merge them into a single sense.

This follows other dictionaries e.g., 
https://www.merriam-webster.com/dictionary/classic
https://www.oed.com/dictionary/classic_adj

\newpage

## Task 5 — 'be full'/'be well'

**Issue:** [#1039](https://github.com/globalwordnet/english-wordnet/issues/1039)  \
**Category:** deprecation  \
**Size:** medium  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1062](https://github.com/globalwordnet/english-wordnet/pull/1062)

**Synset**
 oewn-01190875-v (Interlingual Index: i27559)
(v) **be full** be sated, have enough to eat “I'm full--don't give me any more beans, please”

 oewn-00065266-v (Interlingual Index: i22104)
(v) **be well** be healthy; feel good “She has not been well lately”

**Motivation**
These aren't verbs but adjectives. The synsets should be deprecated by the adjective synsets

\newpage

## Task 6 — fox/weasel/snake

**Issue:** [#1312](https://github.com/globalwordnet/english-wordnet/issues/1312)  \
**Category:** other  \
**Size:** tiny  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1320](https://github.com/globalwordnet/english-wordnet/pull/1320)

fox/weasel/snake

we have three words with similar meanings, but they end up in quite different parents of the hierarchy

fox, dodger, slyboots  `a shifty deceptive person` [U+27EA]i89564[U+27EB] under beguiler [U+27EA]i89432[U+27EB] 
under unwelcome person [U+27EA]i87251[U+27EB] under doer [U+27EA]i88069[U+27EB]

weasel `a person who is regarded as treacherous or sneaky` [U+27EA]i94062[U+27EB] directly under  doer [U+27EA]i88069[U+27EB]

snake, snake in the grass 'a deceitful or treacherous person' [U+27EA]i93095[U+27EB] under bad person [U+27EA]i88407[U+27EB] under  doer [U+27EA]i88069[U+27EB]

I think all three should be under beguiler [U+27EA]i89432[U+27EB]. 

**Motivation**

They all have similar meanings, so should be close to each other.

\newpage

## Task 7 — 'bendy'

**Issue:** [#1400](https://github.com/globalwordnet/english-wordnet/issues/1400)  \
**Category:** other  \
**Size:** small  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1405](https://github.com/globalwordnet/english-wordnet/pull/1405)

**Synset**
01025680-a

**Lemma**
'bendy'

**Motivation**
It is weird that we have 'bendy tree' but not 'bendy'. It is a synonym of 'flexible'

\newpage

## Task 8 — 'pair'< 'building'

**Issue:** [#1221](https://github.com/globalwordnet/english-wordnet/issues/1221)  \
**Category:** relation  \
**Size:** tiny  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1330](https://github.com/globalwordnet/english-wordnet/pull/1330)

**Existing relation**
oewn-07993383-n (Interlingual Index: i79228)
(n) [pair](https://en-word.net/lemma/pair) two people considered as a unit

oewn-07989688-n (Interlingual Index: i79211)
(n) [building](https://en-word.net/lemma/building) the occupants of a building “the entire building complained about the noise”

**Motivation**
These two shouldn't be related directly. 'pair' has another hypernym so this can be removed without any other changes

\newpage

## Task 9 — 'country' vs 'countryside'

**Issue:** [#1386](https://github.com/globalwordnet/english-wordnet/issues/1386)  \
**Category:** relation  \
**Size:** small  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1394](https://github.com/globalwordnet/english-wordnet/pull/1394)

**Synsets**
```
08662297-n: country, rural area
    an area outside of cities and towns
    (his poetry celebrated the slower pace of life in the country)
    hypernym: 08591861-n (geographical area, geographic area, geographical region, geographic region)
    mero_part: 08633625-n (pasture, pastureland, pasture land, grazing land, lea, ley)
    hyponym: 08519693-n (backwoods, back country, boondocks, hinterland); 08587011-n (farmland, farming area); 08662608-n (countryside); 08662679-n (scrubland); 08662787-n (weald); 08662893-n (wold)
```

```
08662608-n: countryside
    rural regions
    hypernym: 08662297-n (country, rural area)
```

**Motivation**
'countryside' is a hyponym of 'country' but the definition is nearly the same as the synonym 'rural region'.

All of the examples clearly have substitution without loss of meaning
>  for this one weekend in the country(side)
> but Mr._Flannagan thought the country(side) would be safer in_case_of war

\newpage

## Task 10 — 'pshaw' and similar

**Issue:** [#1368](https://github.com/globalwordnet/english-wordnet/issues/1368)  \
**Category:** relation  \
**Size:** medium  \
**Gold fix (moderator only):** [https://github.com/globalwordnet/english-wordnet/pull/1385](https://github.com/globalwordnet/english-wordnet/pull/1385)

**Synset**
```
01066084-v: pooh-pooh
    express contempt about
    hypernym: 00942415-v (express, verbalize, verbalise, utter, give tongue to)
```

**Lemma**
'pshaw', 'pish', 'pooh'

**Motivation**
These occur quite frequently as interjections (so not in our scope), but also rarely as verbs, for example:

> It cannot be pished and pshawed away, by you or me or another. "Margarita's Soul" by Ingraham Lovell

It was also the answer to a popular word game, so we should have it!

\newpage
