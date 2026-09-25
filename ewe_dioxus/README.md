# ewe-dioxus

A [Dioxus](https://dioxuslabs.com/) web interface for wordnets built in the
[Global Wordnet Association](https://globalwordnet.github.io/) family of formats. This
crate is the frontend/fullstack app that powers [https://en-word.net/](https://en-word.net/)
(the [Open English Wordnet](https://github.com/globalwordnet/english-wordnet)), letting
users search and browse lemmas, synsets, and their relations, and — behind the `edit`
feature — edit them directly in the browser. It's built on the generic `ewe_lib` data
model, so it's also configurable to serve any other Global Wordnet Association wordnet
under a different name, id prefix, logo, and theme; see [Configuration](#configuration-settingstoml).

It is one member of the `ewe` Cargo workspace:

- `ewe_lib` — the core Wordnet data model and storage (ReDB-backed lexicon), plus the
  automaton engine that applies edits.
- `ewe_cli` — a command-line client for querying and editing the Wordnet.
- `ewe_dioxus` (this crate) — the web/desktop UI, built with Dioxus.

## Prerequisites

- A recent Rust toolchain (`rustup`).
- The Dioxus CLI:

  ```bash
  cargo install dioxus-cli
  ```

- A local checkout of a Wordnet's YAML source (e.g. [english-wordnet](https://github.com/globalwordnet/english-wordnet)), if you want the database built automatically (see [Configuration](#configuration-settingstoml) below).

## Project layout

```
ewe_dioxus/
├─ Cargo.toml         # Dependencies and platform feature flags (web/desktop/mobile/server/edit/oewn)
├─ Dioxus.toml        # Dioxus build/app configuration (title, web resources)
├─ settings.toml      # Runtime configuration for this app — see below (git-ignored)
├─ english-wordnet-settings.toml  # Example settings.toml for an Open English Wordnet deployment
├─ downloads.toml     # Optional: configures the Downloads page (see below)
├─ openapi.yaml       # OpenAPI spec, served as the "JSON API documentation" link in the footer
├─ assets/            # Static assets (favicon, bundled logo, styling)
└─ src/
   ├─ main.rs          # App entrypoint, routes, and the fullstack server bootstrap
   ├─ db.rs             # Opens the lexicon and corpus databases, rebuilding if stale (see below)
   ├─ settings.rs        # `EweSettings` struct — loads and parses settings.toml
   ├─ downloads_config.rs # Parses downloads.toml for the Downloads page
   ├─ backend/            # Server-only logic: API/RDF/XML endpoints (`#[get]` server functions)
   │  ├─ api.rs             # JSON API: lemma/synset lookup, autocomplete
   │  ├─ rdf.rs              # Content negotiation, plus RDF/XML and Turtle export
   │  ├─ xml.rs               # WN-LMF XML export
   │  ├─ senses.rs            # Corpus lookups (where a sense occurs, KWIC concordance)
   │  └─ edit.rs               # `edit`-feature-only: apply/save/revert/validate endpoints
   ├─ components/          # Shared UI components (synsets, relations, subcat frames, search, ...)
   └─ views/                # Route-level views (Home, ByLemma, BySynset, History) and the shared layout
```

## Configuration (`settings.toml`)

The app looks for a `settings.toml` file in the current working directory when it starts. If the file is missing, it falls back to generic defaults described below. `settings.toml` is git-ignored, since it's environment-specific (absolute paths, per-deployment branding); [`english-wordnet-settings.toml`](./english-wordnet-settings.toml) is checked in as a copy-and-rename starting point for an Open English Wordnet deployment.

Every path-valued key below (`database`, `wordnet_source`, `corpus_database`, `corpus_source`, `logo`) is resolved relative to `settings.toml`'s own directory, not the process's working directory - so a project folder with its own `settings.toml` and relative paths (e.g. `wordnet_source = "src/yaml"`) works the same no matter where it's loaded from. This is what makes the desktop app's project-folder picker (below) work.

```toml
database = "wordnet.db"
wordnet_source = "/path/to/english-wordnet/src/yaml/"
corpus_database = "corpus.db"
corpus_source = "/path/to/corpus.yaml"
logo = "assets/english.svg"
project_name = "My Wordnet"
id_prefix = "oewn"
contact_email = "me@example.org"
source_url = "https://github.com/globalwordnet/english-wordnet"
base_url = "https://example.org/"
footer = """
<p>...</p>
"""

[theme]
primary = "#002868"
accent = "#bf0a30"
```

| Key                 | Type             | Default | Description |
|---------------------|------------------|---------|-------------|
| `database`          | string           | `"wordnet.db"` | Path to the ReDB database file that the app reads from (and, if it needs rebuilding, writes to). |
| `wordnet_source`    | string, optional | unset | Path to a directory of Wordnet YAML source files (e.g. from the [english-wordnet](https://github.com/globalwordnet/english-wordnet) repo). Leave unset if you already have a database and don't want it rebuilt. |
| `corpus_database`   | string           | `"corpus.db"` | Path to the corpus database file backing the "where does this sense occur" lookups (see [Corpus lookups](#corpus-lookups) below). |
| `corpus_source`     | string, optional | unset | Path to a Teanga-format corpus YAML file. Leave unset if you already have a corpus database and don't want it rebuilt. |
| `id_prefix`         | string           | `"oewn"` | Prefix used for synset/entry ids in XML/RDF/Turtle export and in id lookups (e.g. `oewn-00001740-n`), and to derive the corpus's sense-key layer name (`{id_prefix}_key`). Set this to your own project's id prefix if you're not the Open English Wordnet. |
| `contact_email`     | string, optional | unset | Recorded in exported WN-LMF XML's `<Lexicon email="...">` attribute. |
| `source_url`        | string, optional | unset | Recorded in exported WN-LMF XML's `<Lexicon url="...">` attribute. |
| `base_url`          | string, optional | unset | This deployment's own public base URL (e.g. `https://en-word.net`), used to build absolute URLs in `/sitemap.xml` and the `Sitemap:` line in `/robots.txt`. Neither is served with real content while this is unset. |
| `logo`              | string           | `"assets/gwa.svg"` | Path to an SVG logo file, read from disk and inlined directly into the page. |
| `[theme]`           | table, optional  | all unset | CSS custom-property overrides applied on top of the bundled default `theme.css` — see [Styling](#styling) below. Every key is optional and independent; unset ones fall back to the compiled-in default via the normal CSS cascade. Color keys (`primary`, `accent`, `text`, `text_secondary`, `text_muted`, `text_dim`, `text_faint`, `text_mute`, `text_strong`, `text_on_dark`, `border`, `border_light`, `surface_light`, `surface_hover`, `surface_dark`) must be CSS hex colors (`#rgb`, `#rgba`, `#rrggbb`, or `#rrggbbaa`). Font keys (`font_body`, `font_heading`, `font_heading_weight`, `font_mono`) are CSS font-family/weight values. `settings.toml` fails to load with a descriptive error if any key doesn't match. |
| `project_name`      | string           | `"EWE Wordnet Editor"` | Shown as the `<h1>` next to the logo, and as the exported XML's `<Lexicon label="...">`. |
| `tagline`           | string           | a generic tagline | Short line shown centered on the home page, below the search box. |
| `intro`             | string           | generic intro HTML | Introduction HTML shown centered on the home page, below the tagline. |
| `footer`            | string           | a generic credits footer | Raw HTML rendered as-is (via `dangerous_inner_html`) in the page footer. |
| `disable_auto_reload` | bool           | `false` | If true, never rebuild `database`/`corpus_database` just because a source file is newer — they're still built if missing. Useful to skip a slow source scan on startup with very large sources. |
| `lexicon_cache_mb`  | integer          | `128` | Bounds the lexicon database's in-memory page cache (redb otherwise defaults to 1GiB regardless of file size). |

`logo` is read from disk (and inlined into the page) via the same server function that carries `project_name`/`footer` (`backend::api::get_branding`) rather than bundled at build time via Dioxus's `asset!` macro, so you can rebrand a running deployment (swap the logo file, or repoint the path in `settings.toml`) without rebuilding or restarting the app. `[theme]` overrides ride along in that same struct and are applied at runtime via `document.documentElement.style.setProperty(...)` — see [Styling](#styling) below.

`wordnet.db` and `corpus.db` are git-ignored — every developer builds their own copy locally.

Two more optional, top-level config files, both git-ignored example-driven:

- [`downloads.toml`](./downloads.toml) (template: `downloads.toml.example`) — configures the Downloads page: a `downloads_dir` plus a list of releases, each with files whitelisted for serving at `/downloads/{filename}`. If the file is missing, the Downloads page just has nothing to list.
- [`openapi.yaml`](./openapi.yaml) — the OpenAPI spec served at the "JSON API documentation" link in the footer.

## The Wordnet database

The server opens the lexicon database lazily, on first request, via `src/db.rs`. Every time it does, it checks:

- if `database` doesn't exist yet, or
- if `wordnet_source` is set and any file in it (or the sibling `deprecations.csv`) has a newer modification time than `database`,

and if either is true, it rebuilds the database from `wordnet_source` before opening it. Otherwise it just opens the existing file. In practice this means you can edit the Wordnet YAML source and the next request after a restart will pick up the changes automatically — there's no separate load/reload step to run.

If no working database can be opened at all (no `settings.toml`, or a `wordnet_source` that doesn't exist), every route shows a setup screen instead of the normal page content (`components::setup_needed`, wired into `views::wn_layout`) rather than a broken page. On `web`, this just explains what to add to `settings.toml`, since there's no interactive way to fix it from a browser. On `desktop` (with the `edit` feature, which desktop already turns on by default), it offers a native "Open Folder" button (via `rfd`) under an "Open Wordnet Folder" heading; the chosen folder must itself contain a `settings.toml` (see [Configuration](#configuration-settingstoml) above for the format - e.g. `~/p/globalwordnet/english-wordnet/settings.toml` for a real Open English Wordnet checkout). Picking a valid folder loads that `settings.toml`, rebuilds the lexicon (and corpus, if configured) from it in the background, and hot-swaps everything - branding included - into the running server once done, with a progress bar and no restart needed (`backend::setup::configure_wordnet_source`). This is session-only and deliberately not persisted anywhere (in particular, it never touches the app's own `settings.toml`) - relaunching the app starts unconfigured again and needs the folder picked once more.

## Corpus lookups

Independently of the lexicon, the app can open a [Teanga](https://github.com/TeangaNLP/teanga.rs)-backed corpus database (`corpus_database`, optionally rebuilt from `corpus_source`) to answer "where does this sense occur" queries — shown on synset pages as keyword-in-context (KWIC) concordance lines. This is entirely supplementary: if the corpus fails to open, the app logs it and carries on without those lookups. The corpus is expected to carry a `{id_prefix}_key`-named layer (see `id_prefix` above) tagging tokens with prefixed sense keys; `src/db.rs` builds a search index on that layer on first use so lookups don't have to scan every document.

## Running the interface with Dioxus

This crate uses Dioxus's `fullstack` feature, so `dx serve` builds and runs both the server (which exposes the `/api/...` server functions in `src/backend/api.rs`, plus the RDF/XML/Turtle export routes) and the client in one step.

From the `ewe_dioxus` directory:

```bash
dx serve
```

By default this serves the `web` platform, read-only. To target a different platform, pass `--platform`:

```bash
dx serve --platform desktop
dx serve --platform web
```

To enable the in-browser editor, pass `--features edit`:

```bash
dx serve --features edit
dx serve --platform desktop --features edit
```

(`desktop` builds already include the `edit` feature in the client binary by default, since desktop is a trusted, single-user deployment — but `dx serve --platform desktop` does *not* thread that through to the separately-built server binary that hosts `edit`'s backend routes, so pass `--features edit` explicitly on the `dx serve` command regardless of platform, or every edit-mode route will 404 even though the UI renders edit controls.)

(`mobile` is also defined as a Cargo feature but is not currently wired up as a first-class target here.)

To bake Open English Wordnet's own navy/red in as the compiled-in default palette (instead of this crate's generic GWA colours), pass `--features oewn`:

```bash
dx serve --features oewn
dx build --platform web --release --features oewn
```

This is for the en-word.net deployment specifically — see [Styling](#styling) below for why it exists.

For a production build:

```bash
dx build --platform web --release
```

## Styling

Layout and component CSS lives under `assets/styling/` (`main.css`, `navbar.css`, `synset.css`, `display_options.css`, `download_links.css`), bundled at build time and linked via `document::Link` in `src/main.rs`. None of it hardcodes colours or fonts directly — every rule references a CSS custom property (`var(--color-...)`, `var(--font-...)`) defined in `assets/styling/theme.css`, which is bundled and linked the same static way as `main.css` and never changes at runtime.

Per-deployment restyling is a `[theme]` table in `settings.toml` (see [Configuration](#configuration-settingstoml) above) overriding a handful of those custom properties — not a whole replacement stylesheet. `src/views/wn_layout.rs` applies whichever ones are set via one `document.documentElement.style.setProperty(...)`/`removeProperty(...)` call (`document.documentElement`, not some element further down the page, because `main.css`'s own `body` rule reads `var(--font-body)`/`var(--color-accent)`/`var(--color-text)` directly, and `body` is an ancestor of everything else rendered — an override set any lower would never reach it). To restyle the site, add the properties you want to override under `[theme]` in `settings.toml`; anything left out keeps `theme.css`'s compiled-in default via the normal CSS cascade. This project does not use Tailwind.

`font_body`/`font_heading`/`font_mono` only take effect if the family they name is actually loaded somewhere — a `[theme]` override can't add its own `@font-face` rule (there's no per-deployment stylesheet left to put one in), only pick among the families `theme.css` already bundles: `"Arvo"` (400/700, `--font-heading`'s default), `"Dosis"` (400, `--font-body`'s default), `"Scope One"` (400, `--font-mono`'s default), and `"Uncial Antiqua"` (400 only, not used by default — set `font_heading_weight = "400"` alongside it, since requesting 700 on a single-weight face triggers the browser's synthetic bold). Naming any other family just falls back to its generic CSS keyword (`serif`/`sans-serif`/`monospace`) once the requested one fails to load — silently, with no error at `settings.toml` load time, since `ThemeOverrides::validate` only rejects characters that could break out of the generated JS, not unbundled font names.

That `[theme]` override only takes effect once the client's async `Branding` fetch resolves and runs its `document::eval`, so on a slow connection the page visibly flickers from `theme.css`'s compiled-in default to the override — noticeable on en-word.net, whose `english-wordnet-settings.toml` overrides `primary`/`accent` back to Open English Wordnet's navy/red. The `oewn` Cargo feature (`--features oewn`, see above) avoids this for that specific case by swapping which stylesheet `src/main.rs`'s `THEME_CSS` bundles at compile time: `assets/styling/theme-oewn.css` (OEWN's navy/red) instead of `assets/styling/theme.css` (this crate's generic GWA default). With the compiled-in default already correct, there's no visible flip — the runtime `[theme]` override in `settings.toml` becomes redundant (harmless to leave in place) rather than load-bearing. This only covers the palette in `theme-oewn.css`, not `logo`/`project_name`/etc., which still only ever come from `settings.toml`.

## Routes

### Pages

Defined in `src/main.rs`, rendered inside the shared `WNLayout` (logo, title, footer) in `src/views/wn_layout.rs`. Each page also sets its own browser tab title via `document::Title` (e.g. `dog - EWE Wordnet Editor`, `00001740-n - EWE Wordnet Editor`), composed from `project_name` (shared via context from `WNLayout`, see `src/components/branding.rs`) and the page's lemma/synset id/section name; `Dioxus.toml`'s static `title` is only the fallback shown before that resolves.

- `/` — `Home` (search box)
- `/view/lemma/:lemma` — `ByLemma` (lists synsets for a given lemma, grouped by part of speech)
- `/view/synset/:synset` — `BySynset` (a single synset; under the `edit` feature, this is also where editing happens)
- `/view/senses/:id?:page` — `BySenses` (paginated KWIC concordance for a synset, from the corpus — see [Corpus lookups](#corpus-lookups))
- `/downloads` — `Downloads` (release archives listed in `downloads.toml`; not built on `desktop`)
- `/history` — `History` (the change log of edits applied so far — see [Editing](#editing-the-edit-feature) below)

### Content-negotiated lookup

- `/synset/{id}` and `/lemma/{lemma}` — redirect to one of the routes below based on the `Accept` header (`text/html`, `application/rdf+xml`, `text/turtle`, `application/xml`, or `application/json`).

### Machine-readable exports

For both `synset/{id}` and `lemma/{lemma}`:

- `/api/synset/{id}`, `/api/lemma/{lemma}` — JSON (full synset documents; `/api/by_lemma/{lemma}` returns just the synset ids for a lemma).
- `/rdf/synset/{id}`, `/rdf/lemma/{lemma}` — RDF/XML.
- `/ttl/synset/{id}`, `/ttl/lemma/{lemma}` — Turtle.
- `/xml/synset/{id}`, `/xml/lemma/{lemma}` — [WN-LMF](https://globalwordnet.github.io/schemas/) XML.

These are also linked from the "Download As" links shown on the synset/lemma pages.

### Search

`/api/autocomplete/{query}` (used by the search box) matches lemmas, synset ids (bare, e.g. `00001740-n`, or prefixed with the configured `id_prefix`), and ILI identifiers (e.g. `i35545`), routing suggestions to the right page based on what matched.

### Corpus lookups

- `/api/senses/{id}/concordance?page` — one page of KWIC concordance lines for a synset.
- `/api/senses/{id}/count` — a fast, approximate count of how many documents contain the sense.
- `/api/senses/{id}` — the ids of every document containing the sense.

### Assets

- `/downloads/{filename}` — serves a whitelisted release file from `downloads.toml` (see [Configuration](#configuration-settingstoml)); not available on `desktop`.

## Editing (the `edit` feature)

Behind `--features edit`, `BySynset` pages gain a pencil toggle that turns the whole synset editable in place: lemmas, definitions, examples, relations (with lemma-level source/target pickers), and ILI/Wikidata identifiers, plus buttons to create a new synset or delete the current one. Every field's pending edits are batched and applied together — via `ewe_lib::automaton::apply_automaton` — when the accept (✓) button is clicked, or discarded together on reject (×). Backend routes for all of this live in `src/backend/edit.rs`.

Every applied batch is recorded in an append-only change log, viewable at `/history`, and shows up as an "unsaved changes" toast until it's written back out. From that toast (or independently, via the "Validate" footer link):

- **Save** runs `ewe_lib::validate::validate` first; if there are validation errors, they're shown with a "save anyway" option, otherwise it writes the current database state back out to `wordnet_source` as YAML.
- **Revert** discards every edit since the last save and rebuilds the database fresh from `wordnet_source`.
- **Validate** can be run standalone at any time, independent of saving.

None of this touches `wordnet_source` until you explicitly save — edits only ever land in the `database` file (and the change log) until then.
