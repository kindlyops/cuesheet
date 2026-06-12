# Cuesheet — Build Plan

A small cross-platform GUI app (Tauri 2) that ports the cuesheet functionality from
[`vbs plt build`](https://github.com/kindlyops/vbs/pull/887): open or drag-and-drop a
purple playlist file, generate a PDF cuesheet with Typst, and save it via the OS-native
save dialog.

## Decisions (from interview)

| Topic | Decision |
|---|---|
| Core logic language | Port parsing + cuesheet generation from Go to Rust (no sidecar) |
| Typst | Linked as a Rust library crate (in-process compile, no bundled CLI binary) |
| Scope | Fully offline — cuesheet built only from data inside the playlist ZIP |
| App name | **Cuesheet**, bundle id `com.kindlyops.cuesheet` |
| PDF output | Verbatim port of the vbs Typst template (Helvetica/Arial, teal `#235a68` accents) |
| UX flow | Drop/browse → generate → native save dialog opens immediately |
| App design | Tufte-style window design with Fantastic Mr Fox color palette |
| Auto-update | Tauri built-in updater against GitHub Releases (`latest.json`) |
| Code signing | Not yet — pipeline scaffolds Apple notarization + Windows Authenticode, activated automatically when secrets are added |
| Targets | macOS (universal: aarch64 + x86_64) and Windows (x64) installers |

## 1. Goals and non-goals

**Goals**

- One-screen app: a large affordance to browse for a playlist, plus whole-window drag-and-drop.
- Parse the purple playlist export (ZIP + manifest.json + SQLite, schema version 14) entirely offline.
- Emit a `cuesheet.typ` byte-identical (modulo timestamps) to vbs PR #887 and compile it to PDF in-process with the typst crates.
- Open the OS-native save dialog prefilled with `<playlist-name>-cuesheet.pdf`.
- Hexagonal architecture: the entire domain is a pure Rust crate testable headlessly in CI.
- Fully automated tag-driven release pipeline producing `.dmg` (macOS) and NSIS `.exe` + `.msi` (Windows), plus updater artifacts.
- Complete third-party license attribution (typst is Apache-2.0; full notice bundle generated at build time).

**Non-goals**

- No media download, ffmpeg cutting, or media-API resolution (that's the rest of `plt build`).
- No playlist editing; read-only input.
- No Linux packaging in the release pipeline (CI tests still run on Linux).

## 2. Architecture (hexagonal)

The domain core is a pure crate with no Tauri, filesystem-dialog, or GUI dependency.
Everything I/O-shaped is a port; adapters live at the edges.

```
                     ┌──────────────────────────────────────────┐
  driving adapters   │              cuesheet-core               │   driven adapters
                     │                                          │
 Tauri command  ───▶ │  ports (traits):                         │
 (generate_cuesheet) │   PlaylistSource   ──────────────────────│──▶ ZipPlaylistSource (zip + rusqlite)
                     │   PdfCompiler      ──────────────────────│──▶ TypstCompiler (typst crate + embedded fonts)
 CLI test harness ─▶ │   Clock            ──────────────────────│──▶ SystemClock / FixedClock (tests)
 (xtask/golden tests)│                                          │
                     │  domain: Playlist, Cue, CueMarker,       │
                     │  Ticks, EndAction, TypstTemplate         │
                     └──────────────────────────────────────────┘
```

- **Driving side:** the Tauri command handler is a thin adapter — it receives a path (from
  dialog or drop event), calls `core::generate(path) -> Result<GeneratedCuesheet>`, and hands
  bytes to the save-dialog adapter. A tiny dev CLI (`cargo run -p cuesheet-cli -- <file>`)
  drives the same core for golden tests and local debugging without a GUI.
- **Driven side:** `PlaylistSource` abstracts reading the ZIP/SQLite; `PdfCompiler` abstracts
  Typst so template tests can snapshot the `.typ` source without compiling, and compile tests
  can run against the real typst crate.
- **Frontend:** purely presentational. All state transitions (idle → generating → saved/error)
  come from backend events; the UI holds no business logic.

### Workspace layout

```
cuesheet/
├── Cargo.toml                  # workspace
├── crates/
│   ├── cuesheet-core/          # pure domain: parse, model, typst template emit
│   │   ├── src/
│   │   │   ├── model.rs        # Playlist, Cue, CueMarker, Ticks, EndAction
│   │   │   ├── parse/          # manifest.json + SQLite extraction (port impl kept separate)
│   │   │   ├── template.rs     # verbatim port of the vbs cuesheet.typ emitter
│   │   │   └── ports.rs        # PlaylistSource, PdfCompiler, Clock traits
│   │   └── tests/              # unit + golden-file tests, fixture builder
│   ├── cuesheet-typst/         # PdfCompiler adapter: typst crate, font embedding, World impl
│   └── cuesheet-cli/           # headless driving adapter for CI/golden tests
├── src-tauri/                  # Tauri app: commands, dialog/drop adapters, updater config
├── src/                        # frontend (Svelte 5 + TypeScript + Vite)
├── docs/PLAN.md
├── .github/workflows/{ci.yml, release.yml}
└── about.toml                  # cargo-about license bundle config
```

## 3. Core domain port (from vbs PR #887)

### Input format

Purple playlist export = ZIP archive containing:

- `manifest.json` — requires `userDataBackup.databaseName` (string) and
  `userDataBackup.schemaVersion` (must be **14**; reject others with a clear message).
- SQLite database (filename from the manifest) with tables `Tag`, `TagMap`, `PlaylistItem`,
  `PlaylistItemLocationMap`, `Location`, `IndependentMedia`, `PlaylistItemMarker`.
- Embedded media files (thumbnails / independent images) referenced by `FilePath`.

### Extracted model

- **Playlist:** name (`Tag` where `Type = 2`), language.
- **Items** in playback order: `Position`, `PlaylistItemId`, `Label`, `StartTrimTicks`,
  `EndTrimTicks`, `EndAction`, `ThumbnailFilePath` (nullable).
- **Locations:** `MajorMultimediaType`, `BaseDurationTicks`, `KeySymbol`/`Track`/
  `BookNumber`/`ChapterNumber`/`DocumentID` (nullable), `MepsLanguage`, `Type`.
- **Independent media (images):** `DurationTicks`, `OriginalFilename`, `FilePath`,
  `MimeType`, `Hash`.
- **Markers:** `Label`, `StartTimeTicks`, `DurationTicks`, `EndTransitionDurationTicks`.

`Ticks` newtype: 1 tick = 100 ns (10 000 000 ticks/second), with display helpers for
`mm:ss` / `h:mm:ss` formatting matching the vbs output exactly. `EndAction` enum maps the
integer codes to the vbs labels (continue / stop / freeze).

Offline durations come from `BaseDurationTicks` (published media) or `DurationTicks`
(images), adjusted by trim ticks; thumbnails are the images embedded in the ZIP, extracted
to a session temp dir for Typst to reference.

### Typst template (verbatim)

`template.rs` reproduces the vbs `cuesheet.typ` generator exactly: US Letter, 1.5 cm
margins, footer rule + page numbers, Helvetica Neue/Arial 10 pt stack, header band with
playlist name + metadata line (language, cue count, total duration), and the five-column
table (cue number in `rgb("#235a68")`, 2 cm thumbnail, label + filename, duration with
sparkline, end-action label). Fields that only exist after media processing (clip paths,
cut resolution) render the same way vbs renders them when typst-only output is produced —
verified against golden fixtures generated from the vbs CLI.

### Typst as a library

`cuesheet-typst` implements typst's `World` trait over an in-memory file map: the generated
`.typ` source plus extracted thumbnail bytes, no real filesystem root needed. Fonts: since
the template asks for Helvetica Neue/Arial, we embed **Liberation Sans** (metric-compatible
with Arial, SIL OFL 1.1) so output is deterministic on every platform and in CI, while also
exposing system fonts so macOS users get genuine Helvetica Neue when present. Typst crate
version is pinned; its Apache-2.0 notice and the font OFL text ship in the license bundle.

## 4. GUI design

**Stack:** Tauri 2, Svelte 5 + TypeScript + Vite. Plugins: `dialog` (open/save),
`updater`, `process`. Drag-and-drop via Tauri's native window drop events (works for files
dragged from Finder/Explorer, unlike HTML5 DnD).

**Layout (Tufte-style):** generous whitespace, a single centered column, ET Book–style
serif stack (`et-book, Palatino, Georgia, serif` — ET Book is free, included with notice),
hairline rules, sidenote-styled hints ("or drop a playlist anywhere on this window"),
no chrome beyond the essentials.

**Fantastic Mr Fox palette (design tokens):**

| Token | Hex | Use |
|---|---|---|
| `--cream` | `#F7EED7` | window background |
| `--fox` | `#C95B0C` | primary button, drop-zone highlight |
| `--mustard` | `#E3A72F` | hover/active accents, progress |
| `--russet` | `#7A4419` | rules, secondary text |
| `--ink` | `#33271C` | body text |
| `--apple-cider` | `#EFD9A7` | success state wash |

**States:**

1. **Idle** — large fox-orange "Choose a playlist…" button; whole window is a drop target;
   dropping anywhere highlights the window border in `--fox`.
2. **Generating** — button swaps to an indeterminate progress treatment (parsing →
   compiling), still on-palette; sub-second for typical playlists but visible feedback regardless.
3. **Save** — native save dialog opens automatically, default name
   `<playlist-name>-cuesheet.pdf`, remembering the last-used directory.
4. **Done** — quiet confirmation line with a "Reveal in Finder/Explorer" link and
   "Generate another" returning to idle.
5. **Error** — plain-language message (not a stack trace) for the known failure classes:
   not a ZIP, missing/invalid manifest, unsupported schema version, missing tables,
   typst compile failure; with a "details" disclosure for the underlying error.

**Polish items:** custom app icon (fox-orange motif, full macOS/Windows icon set via
`tauri icon`), window min-size and centered default, native menu with About/Check for
Updates/Quit, About window showing version + bundled third-party license text, hidpi
assets, reduced-motion-respecting transitions.

## 5. Licensing and attribution

- `cargo-about` generates `THIRD_PARTY_LICENSES.html` at build time from the full Rust
  dependency graph (typst Apache-2.0 + its tree); frontend deps covered via
  `license-checker` output merged into the same bundle.
- Font licenses (Liberation Sans OFL 1.1, ET Book license) included verbatim.
- The bundle is embedded in the app (About window) **and** shipped inside the installers.
- Repo `LICENSE` (Apache-2.0, already present) referenced in the About window.

## 6. CI and release pipeline

### `ci.yml` (every push/PR)

- Linux runner: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace`
  (core parsing, golden `.typ` snapshots, PDF smoke-compile via the typst crate — all headless).
- `svelte-check` + frontend unit tests (Vitest) + `eslint`/`prettier` check.
- A cross-platform build sanity job (macOS + Windows runners) running `tauri build
  --no-bundle` to catch platform-specific compile breaks before release day.

### `release.yml` (on tag `v*`)

1. Matrix: `macos-latest` (universal-apple-darwin) and `windows-latest` (x64), using
   `tauri-apps/tauri-action` to build `.dmg`/`.app` and NSIS `.exe` + `.msi`.
2. **Signing, gated on secret presence** (steps are written now, no-op until secrets exist):
   - macOS: `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_ID`,
     `APPLE_PASSWORD`, `APPLE_TEAM_ID` → Developer ID signing + notarytool stapling.
   - Windows: `WINDOWS_CERTIFICATE`, `WINDOWS_CERTIFICATE_PASSWORD` → Authenticode.
   - Without secrets the job still produces working unsigned installers and the release
     notes state they're unsigned.
3. **Updater:** `TAURI_SIGNING_PRIVATE_KEY` (+ password) signs update bundles;
   `tauri-action` assembles `latest.json` pointing at the GitHub Release assets. The
   updater keypair must be generated once (`tauri signer generate`) and added as secrets
   before the first release — documented in `docs/RELEASING.md`. The updater endpoint in
   `tauri.conf.json` targets `https://github.com/kindlyops/cuesheet/releases/latest/download/latest.json`.
4. Release is created as a **draft** with generated notes + license bundle attached;
   publishing the draft is the single manual gate.
5. Version bumping via one source of truth (`tauri.conf.json` + workspace version kept in
   sync by an `xtask bump` helper); tags drive everything else.

## 7. Testing strategy

| Layer | What | Where it runs |
|---|---|---|
| Domain unit tests | ticks math, end-action mapping, trim/duration calculation, name extraction | every CI run, Linux |
| Parser tests | fixture builder that constructs in-memory playlist ZIPs (manifest + SQLite + images) covering happy path and every malformed-input class | CI, Linux |
| Golden tests | `.typ` output snapshot-compared against fixtures captured from the vbs CLI to guarantee the "verbatim" requirement | CI, Linux |
| PDF smoke tests | compile golden `.typ` with the typst crate, assert page count + non-empty output (PDF bytes aren't byte-stable, so structure-level asserts) | CI, Linux |
| Adapter tests | Tauri command layer tested against mock ports | CI, Linux |
| Frontend | Vitest component tests for the state machine (idle/generating/done/error) with mocked `invoke` | CI, Linux |
| Build sanity | `tauri build --no-bundle` on macOS + Windows runners | CI, mac/win |

The hexagonal split is what makes every functional test headless: nothing in
`cuesheet-core`/`cuesheet-typst` knows Tauri exists.

## 8. GitHub Pages site (three.js)

A single-page marketing site at `https://kindlyops.github.io/cuesheet/`, quirky and
handmade-feeling, sharing the app's design language.

- **Centerpiece:** a paper-craft, stop-motion-styled autumn diorama — a low-poly **red
  panda** among paper trees and falling leaves, in the Mr Fox palette — rendered with
  three.js. The scene parallax-rotates subtly with scroll and mouse movement; animation
  uses a gentle stop-motion cadence (stepped keyframes at ~12 fps feel) and respects
  `prefers-reduced-motion` with a static hero render fallback.
- **Content below the fold:** what-it-does blurb in the same Tufte typography, app
  screenshots, and macOS/Windows download buttons that resolve the latest GitHub Release
  assets at load time via the public Releases API (no rebuild needed per release).
- **Implementation:** `site/` directory in this repo — Vite + TypeScript + three.js, models
  authored as glTF (low-poly, flat paper-texture materials, hand-built or via Blender
  export committed to the repo). No framework; the page is mostly static HTML/CSS.
- **Deploy:** `pages.yml` workflow builds `site/` and deploys via
  `actions/deploy-pages` on every push to `main` touching `site/`.
- **Budget:** scene kept under ~1 MB of assets, lazy-initialized after first paint, graceful
  no-WebGL fallback to the static render.

## 9. Milestones

1. **M1 — Core port:** workspace scaffold; model + ZIP/SQLite parser; fixture builder;
   unit + parser tests green in CI.
2. **M2 — Typst output:** verbatim template emitter; golden fixtures from vbs; in-process
   PDF compile with embedded fonts; `cuesheet-cli` for headless generation.
3. **M3 — App shell:** Tauri 2 + Svelte scaffold; browse + drag-and-drop; generate →
   native save dialog; error states; Mr Fox/Tufte design system; icon.
4. **M4 — Polish & licensing:** About window, menus, license bundle (cargo-about),
   reveal-in-folder, last-directory memory.
5. **M5 — Pipeline:** ci.yml, release.yml with gated signing + updater, `docs/RELEASING.md`,
   first tagged draft release (unsigned) validating end-to-end installers on both OSes.
6. **M6 — Pages site:** red-panda diorama scene, landing page content, download buttons
   wired to latest release, `pages.yml` deploy workflow.

## 10. Risks / notes

- **Golden parity:** the vbs template emits some fields (clip filenames, resolution) that
  come from the media pipeline; the offline app reproduces vbs's behavior for the
  pre-processing case. Fixtures captured from the actual CLI keep this honest.
- **Schema drift:** only schema version 14 is accepted, same as vbs; newer exports fail
  with an explicit "unsupported schema version" message rather than garbage output.
- **Updater before signing:** Tauri's updater signature is independent of OS code signing,
  so auto-update works even while installers are unsigned; macOS Gatekeeper will still
  warn on first install until notarization secrets are added.
