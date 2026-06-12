# Cuesheet

Drop a playlist, get a beautifully typeset PDF cuesheet.

Cuesheet is a small cross-platform desktop app (macOS and Windows) that ports
the cuesheet generation from [`vbs plt`](https://github.com/kindlyops/vbs)
to a GUI. Open or drag-and-drop a purple playlist export and Cuesheet parses
it offline, typesets a cuesheet with [Typst](https://typst.app) (compiled
in-process — no external tools needed), and offers a native save dialog for
the resulting PDF.

See [docs/PLAN.md](docs/PLAN.md) for the architecture (hexagonal core, ports
and adapters) and the full design.

## Development

Common tasks are driven by [`just`](https://github.com/casey/just):

```sh
just            # list recipes
just test       # run all Rust tests
just check      # fmt + clippy, what CI enforces
just pdf my-playlist.zip   # generate a PDF headlessly
just app        # run the desktop app in dev mode
just site       # run the marketing site locally
just ci         # everything CI runs
```

The domain logic lives in `crates/cuesheet-core` (pure, no GUI deps),
PDF compilation in `crates/cuesheet-typst`, a headless CLI in
`crates/cuesheet-cli`, the Tauri app in `src-tauri/` + `src/`, and the
website in `site/`.

## License

Apache-2.0. Bundled third-party components (Typst and friends) retain their
own licenses; the app ships a generated `THIRD_PARTY_LICENSES.html`.
