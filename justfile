# Cuesheet developer commands. Install just: https://github.com/casey/just

# List available recipes
default:
    @just --list

# Run all Rust tests (core, typst adapter, CLI)
test:
    cargo test --workspace

# Build everything (debug)
build:
    cargo build --workspace

# Format + lint, exactly what CI enforces
check:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings

# Auto-fix formatting
fmt:
    cargo fmt --all

# Regenerate the golden cuesheet.typ after an intentional template change
bless:
    BLESS=1 cargo test -p cuesheet-core --test golden_test

# Generate a cuesheet PDF from a playlist file, headless
pdf playlist *args:
    cargo run -p cuesheet-cli -- {{playlist}} {{args}}

# Rebuild the code-signing setup PDF from its Typst source
signing-doc:
    cargo run -p cuesheet-typst --example typst_to_pdf -- docs/signing-setup.typ docs/signing-setup.pdf

# Run the desktop app in dev mode (hot reload)
app:
    cd src-tauri && cargo tauri dev

# Build the desktop app installers for this machine
app-build:
    cd src-tauri && cargo tauri build

# Install frontend + site npm dependencies
setup:
    npm install
    cd site && npm install

# Frontend checks (type-check + unit tests)
frontend-check:
    npm run check
    npm test

# Build the browser (wasm) engine into the site's public assets
wasm:
    wasm-pack build crates/cuesheet-wasm --target web --release --out-dir ../../site/public/wasm

# Run the marketing site locally (run `just wasm` first for the in-browser app)
site:
    cd site && npm run dev

# Build the marketing site
site-build:
    cd site && npm run build

# Everything CI runs, locally
ci: check test frontend-check site-build
