# ewe-dioxus

A [Dioxus](https://dioxuslabs.com/) web interface for the [Open English Wordnet](https://github.com/globalwordnet/english-wordnet). This crate is the frontend/fullstack app that (will) power [https://en-word.net/](https://en-word.net/), letting users search and browse lemmas, synsets, and their relations — and, since it's built on the generic `oewn_lib` data model, is also configurable enough to serve other Global Wordnet Association wordnets under a different name, logo, and theme.

It is one member of the `ewe` Cargo workspace:

- `oewn_lib` — the core Wordnet data model and storage (ReDB-backed lexicon).
- `ewe_cli` — a command-line client for querying the Wordnet.
- `ewe_dioxus` (this crate) — the web/desktop UI, built with Dioxus.

## Prerequisites

- A recent Rust toolchain (`rustup`).
- The Dioxus CLI:

  ```bash
  cargo install dioxus-cli
  ```

- A local checkout of the [english-wordnet](https://github.com/globalwordnet/english-wordnet) YAML source, if you want the database built automatically (see [Configuration](#configuration-settingstoml) below).

## Project layout

```
ewe_dioxus/
├─ Cargo.toml         # Dependencies and platform feature flags (web/desktop/mobile/server)
├─ Dioxus.toml        # Dioxus build/app configuration (title, web resources)
├─ settings.toml      # Runtime configuration for this app — see below (git-ignored)
├─ english-wordnet-settings.toml  # Example settings.toml for an English Wordnet deployment
├─ assets/            # Static assets (favicon, bundled logo, styling)
└─ src/
   ├─ main.rs          # App entrypoint, routes, and the fullstack server bootstrap
   ├─ db.rs             # Opens the lexicon database, rebuilding it if stale (see below)
   ├─ settings.rs        # `EweSettings` struct — loads and parses settings.toml
   ├─ backend/            # Server-only logic: API/RDF/XML endpoints (`#[get]` server functions)
   │  ├─ api.rs             # JSON API: lemma/synset lookup, autocomplete
   │  ├─ rdf.rs              # Content negotiation, plus RDF/XML and Turtle export
   │  ├─ xml.rs               # WN-LMF XML export
   │  └─ static_files.rs       # Serves `/logo` and `/theme.css` from the paths in settings.toml
   ├─ components/          # Shared UI components (synsets, relations, subcat frames, search, ...)
   └─ views/                # Route-level views (Home, ByLemma, BySynset) and the shared layout
```

## Configuration (`settings.toml`)

The app looks for a `settings.toml` file in the current working directory when it starts. If the file is missing, it falls back to the defaults described below (which reproduce the Open English Wordnet branding). `settings.toml` is git-ignored, since it's environment-specific (absolute paths, per-deployment branding); [`english-wordnet-settings.toml`](./english-wordnet-settings.toml) is checked in as a copy-and-rename starting point.

```toml
database = "wordnet.db"
wordnet_source = "/path/to/english-wordnet/src/yaml/"
logo = "assets/english.svg"
theme = "assets/styling/theme.css"
project_name = "Open English Wordnet"
footer = """
<p>...</p>
"""
```

| Key              | Type             | Default | Description |
|------------------|------------------|---------|-------------|
| `database`       | string           | `"wordnet.db"` | Path to the ReDB database file that the app reads from (and, if it needs rebuilding, writes to). |
| `wordnet_source` | string, optional | unset | Path to a directory of Wordnet YAML source files (from the [english-wordnet](https://github.com/globalwordnet/english-wordnet) repo). Leave unset if you already have a database and don't want it rebuilt. |
| `logo`           | string           | `"assets/english.svg"` | Path to the logo image, read from disk on every request and served at `/logo`. |
| `theme`          | string           | `"assets/styling/theme.css"` | Path to the theme stylesheet (colours and fonts as CSS custom properties — see [Styling](#styling) below), read from disk on every request and served at `/theme.css`. |
| `project_name`   | string           | `"Open English Wordnet"` | Shown as the `<h1>` next to the logo. |
| `footer`         | string           | the Open English Wordnet attribution/credits footer | Raw HTML rendered as-is (via `dangerous_inner_html`) in the page footer. |

Because `logo` and `theme` are read from disk per-request rather than bundled at build time via Dioxus's `asset!` macro, you can rebrand a running deployment (swap the logo file, edit the theme file, or repoint either path in `settings.toml`) without rebuilding or restarting the app.

`wordnet.db` is git-ignored — every developer builds their own copy locally.

## The Wordnet database

The server opens the database lazily, on first request, via `src/db.rs`. Every time it does, it checks:

- if `database` doesn't exist yet, or
- if `wordnet_source` is set and any file in it (or the sibling `deprecations.csv`) has a newer modification time than `database`,

and if either is true, it rebuilds the database from `wordnet_source` before opening it. Otherwise it just opens the existing file. In practice this means you can edit the Wordnet YAML source and the next request after a restart will pick up the changes automatically — there's no separate load/reload step to run.

## Running the interface with Dioxus

This crate uses Dioxus's `fullstack` feature, so `dx serve` builds and runs both the server (which exposes the `/api/...` server functions in `src/backend/api.rs`, plus the RDF/XML/Turtle export routes) and the client in one step.

From the `ewe_dioxus` directory:

```bash
dx serve
```

By default this serves the `web` platform. To target a different platform, pass `--platform`:

```bash
dx serve --platform desktop
dx serve --platform web
```

(`mobile` is also defined as a Cargo feature but is not currently wired up as a first-class target here.)

For a production build:

```bash
dx build --platform web --release
```

## Styling

Layout and component CSS lives under `assets/styling/` (`main.css`, `navbar.css`, `synset.css`, `display_options.css`, `download_links.css`), linked via `document::Link` in `src/main.rs` and per-component `document::Style` includes. None of it hardcodes colours or fonts directly — every rule references a CSS custom property (`var(--color-...)`, `var(--font-...)`) defined in `assets/styling/theme.css`, which is served at the configurable `/theme.css` path described above. To restyle the site, either edit `theme.css` in place or point `theme` in `settings.toml` at a different file with the same custom properties defined. This project does not use Tailwind.

## Routes

### Pages

Defined in `src/main.rs`, rendered inside the shared `WNLayout` (logo, title, footer) in `src/views/wn_layout.rs`:

- `/` — `Home` (search box)
- `/view/lemma/:lemma` — `ByLemma` (lists synsets for a given lemma, grouped by part of speech)
- `/view/synset/:synset` — `BySynset` (a single synset)

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

`/api/autocomplete/{query}` (used by the search box) matches lemmas, synset ids (bare, e.g. `00001740-n`, or `oewn`-prefixed), and ILI identifiers (e.g. `i35545`), routing suggestions to the right page based on what matched.

### Assets

- `/logo`, `/theme.css` — see [Configuration](#configuration-settingstoml) above.
