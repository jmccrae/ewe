# Lexicographer files

Every synset belongs to exactly one of these (the `lexfile` argument to
`apply_automaton`'s `add_synset` action). Nouns and verbs each have their own set;
adjectives and adverbs are each a single catch-all file.

## Noun

- `noun.Tops` — unique beginners for nouns (very general categories)
- `noun.act` — acts or actions
- `noun.animal` — animals
- `noun.artifact` — man-made objects
- `noun.attribute` — attributes of people and objects
- `noun.body` — body parts
- `noun.cognition` — cognitive processes and contents
- `noun.communication` — communicative processes and contents
- `noun.event` — natural events
- `noun.feeling` — feelings and emotions
- `noun.food` — foods and drinks
- `noun.group` — groupings of people or objects
- `noun.location` — spatial position
- `noun.motive` — goals
- `noun.object` — natural objects (not man-made)
- `noun.person` — people
- `noun.phenomenon` — natural phenomena
- `noun.plant` — plants
- `noun.possession` — possession and transfer of possession
- `noun.process` — natural processes
- `noun.quantity` — quantities and units of measure
- `noun.relation` — relations between people, things, or ideas
- `noun.shape` — two- and three-dimensional shapes
- `noun.state` — stable states of affairs
- `noun.substance` — substances
- `noun.time` — time and temporal relations

## Verb

- `verb.body` — grooming, dressing, and bodily care
- `verb.change` — change of size, temperature, intensity, etc.
- `verb.cognition` — thinking, judging, analyzing, doubting
- `verb.communication` — telling, asking, ordering, singing
- `verb.competition` — fighting, athletic activities
- `verb.consumption` — eating and drinking
- `verb.contact` — touching, hitting, tying, digging
- `verb.creation` — sewing, baking, painting, performing
- `verb.emotion` — feeling
- `verb.motion` — walking, flying, swimming
- `verb.perception` — seeing, hearing, feeling
- `verb.possession` — buying, selling, owning
- `verb.social` — political and social activities and events
- `verb.stative` — being, having, spatial relations
- `verb.weather` — raining, snowing, thawing, thundering

## Adjective and adverb

- `adj.all` — general adjectives
- `adj.pert` — relational adjectives (pertains to a noun, e.g. "presidential")
- `adj.ppl` — participial adjectives
- `adv.all` — general adverbs
