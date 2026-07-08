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

- A built Wordnet database (see [Loading the Wordnet database](#loading-the-wordnet-database) below) — the app has nothing to serve without it.

## Project layout

```
ewe_dioxus/
├─ Cargo.toml       # Dependencies and platform feature flags (web/desktop/mobile/server)
├─ Dioxus.toml       # Dioxus build/app configuration (title, web resources)
├─ settings.toml     # Runtime configuration for this app — see below
├─ assets/            # Static assets (favicon, styling)
└─ src/
   ├─ main.rs          # App entrypoint, routes, and the fullstack server bootstrap
   ├─ load.rs           # Standalone `load` binary that builds the Wordnet database
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
| `database`       | string           | Path to the ReDB database file. Used by the `load` binary as the output path, and expected by the server as its input path. |
| `wordnet_source` | string, optional | Path to a directory of Wordnet YAML source files (from the [english-wordnet](https://github.com/globalwordnet/english-wordnet) repo). Only used by the `load` binary; leave unset if you're not (re)building the database. |

> **Note:** the running app currently opens the ReDB database from the fixed path `wordnet.db` in the working directory, rather than reading `settings.database` dynamically. Keep `database = "wordnet.db"` in `settings.toml` (the default) so the path the `load` binary writes to matches what the server reads from.

`wordnet.db` is git-ignored — every developer builds their own copy locally.

## Loading the Wordnet database

Before serving the app for the first time, build the ReDB database from the Wordnet YAML sources using the `load` binary:

1. Clone the [english-wordnet](https://github.com/globalwordnet/english-wordnet) source repo somewhere locally.
2. Point `wordnet_source` in `settings.toml` at its `src/yaml/` directory.
3. Run:

   ```bash
   cargo run --bin load
   ```

This creates (or overwrites) the database at the path given by `database` in `settings.toml`, printing load progress as it goes. You only need to redo this when the source data changes.

## Running the interface with Dioxus

This crate uses Dioxus's `fullstack` feature, so `dx serve` builds and runs both the server (which exposes the `/api/...` server functions in `src/backend/api.rs`) and the client in one step.

From the `ewe_dioxus` directory, with a `wordnet.db` already built:

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
