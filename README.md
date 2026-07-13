# Minisatoshi

Offline-first desktop app for creating and managing Bitcoin vaults with Miniscript.

## Status

- **Sprint 0** — Monorepo scaffold (Tauri 2 + React)
- **Sprint 1** — `policy-engine` + `descriptor-engine`
- **Sprint 2** — `storage` (SQLite) + `wallet-core`

## Tech stack

- Rust, Tauri 2, React, TypeScript
- rust-bitcoin, rust-miniscript
- SQLite (planned, Sprint 2)

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 20+
- Windows: [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)

## Quick start

```bash
# Install Rust dependencies & run tests
cargo test --workspace

# Desktop app
cd apps/desktop
npm install
npm run tauri dev
```

## Project structure

```text
minisatoshi/
├── apps/desktop/          # Tauri 2 + React UI
├── crates/
│   ├── policy-engine/     # JSON policy → Miniscript
│   ├── descriptor-engine/ # Miniscript → descriptor
│   ├── storage/           # SQLite persistence
│   ├── wallet-core/       # wallet + vault lifecycle
│   └── ...                # upcoming modules
├── docs/DEVELOPMENT_PLAN.md
└── tests/vectors/
```

## Policy example

```json
{
  "version": 1,
  "network": "testnet",
  "script_type": "taproot",
  "keys": [
    { "id": "A", "role": "investor", "xpub": "tpub...", "fingerprint": "a1b2c3d4" },
    { "id": "B", "role": "manager",  "xpub": "tpub...", "fingerprint": "e5f6a7b8" },
    { "id": "C", "role": "recovery", "xpub": "tpub...", "fingerprint": "c9d0e1f2" }
  ],
  "policy": {
    "primary": "(A && B) || (A && C)",
    "fallback": { "after": "4y", "allow": "A" }
  }
}
```

## Development plan

See [docs/DEVELOPMENT_PLAN.md](docs/DEVELOPMENT_PLAN.md) for the full roadmap.

## License

Apache-2.0
