# ewe-dioxus

A [Dioxus](https://dioxuslabs.com/) web interface for the [Open English Wordnet](https://github.com/globalwordnet/english-wordnet). This crate is the frontend/fullstack app that (will) power [https://en-word.net/](https://en-word.net/), letting users search and browse lemmas, synsets, and their relations.

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
├─ Cargo.toml       # Dependencies and platform feature flags (web/desktop/mobile/server)
├─ Dioxus.toml       # Dioxus build/app configuration (title, web resources)
├─ settings.toml     # Runtime configuration for this app — see below
├─ assets/            # Static assets (favicon, styling)
└─ src/
   ├─ main.rs          # App entrypoint, routes, and the fullstack server bootstrap
   ├─ db.rs             # Opens the lexicon database, rebuilding it if stale (see below)
   ├─ settings.rs        # `EweSettings` struct — loads and parses settings.toml
   ├─ backend/            # Server-only logic: API endpoints (`#[get]` server functions)
   ├─ components/          # Shared UI components (synsets, relations, subcat frames, ...)
   └─ views/                # Route-level views (Home, ByLemma) and the shared layout/navbar
```

## Configuration (`settings.toml`)

The app looks for a `settings.toml` file in the current working directory when it starts. If the file is missing, it falls back to defaults (`database = "wordnet.db"`, no `wordnet_source`).

```toml
database = "wordnet.db"
wordnet_source = "/path/to/english-wordnet/src/yaml/"
```

| Key              | Type             | Description |
|------------------|------------------|-------------|
| `database`       | string           | Path to the ReDB database file that the app reads from (and, if it needs rebuilding, writes to). |
| `wordnet_source` | string, optional | Path to a directory of Wordnet YAML source files (from the [english-wordnet](https://github.com/globalwordnet/english-wordnet) repo). Leave unset if you already have a database and don't want it rebuilt. |

`wordnet.db` is git-ignored — every developer builds their own copy locally.

## The Wordnet database

The server opens the database lazily, on first request, via `src/db.rs`. Every time it does, it checks:

- if `database` doesn't exist yet, or
- if `wordnet_source` is set and any file in it (or the sibling `deprecations.csv`) has a newer modification time than `database`,

and if either is true, it rebuilds the database from `wordnet_source` before opening it. Otherwise it just opens the existing file. In practice this means you can edit the Wordnet YAML source and the next request after a restart will pick up the changes automatically — there's no separate load/reload step to run.

## Running the interface with Dioxus

This crate uses Dioxus's `fullstack` feature, so `dx serve` builds and runs both the server (which exposes the `/api/...` server functions in `src/backend/api.rs`) and the client in one step.

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

Styling is hand-written CSS under `assets/styling/` (`main.css`, `navbar.css`, `synset.css`), linked directly via `document::Link` in `src/main.rs`. This project does not use Tailwind.

## Routes

Defined in `src/main.rs`:

- `/` — `Home` (search/landing page)
- `/lemma/:lemma` — `ByLemma` (lists synsets for a given lemma)

Both are rendered inside the shared `WNLayout` (navbar) defined in `src/views/wn_layout.rs`.
