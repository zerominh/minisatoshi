# Minisatoshi

Offline-first desktop app for creating and managing Bitcoin vaults with Miniscript.

## Status

**v0.1.0** — Sprint 0–8 complete (MVP hardening + release tooling).

See [CHANGELOG.md](CHANGELOG.md) and [docs/DEVELOPMENT_PLAN.md](docs/DEVELOPMENT_PLAN.md).

## Tech stack

- Rust, Tauri 2, React, TypeScript
- rust-bitcoin, rust-miniscript
- SQLite (watch-only vault storage)

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

### Release build

```bash
cd apps/desktop
npm ci
npm run tauri build
```

Tagging `v*` on GitHub runs [.github/workflows/release.yml](.github/workflows/release.yml) (Windows, macOS, Linux).

## Project structure

```text
minisatoshi/
├── apps/desktop/          # Tauri 2 + React UI
├── crates/
│   ├── policy-engine/     # JSON policy → Miniscript
│   ├── descriptor-engine/ # Miniscript → descriptor
│   ├── storage/           # SQLite persistence
│   ├── wallet-core/       # wallet + vault lifecycle
│   ├── address-engine/
│   ├── blockchain/        # Esplora / Electrum / Sparrow interop
│   ├── psbt-engine/
│   └── vault/
├── docs/
│   ├── DEVELOPMENT_PLAN.md
│   └── policy-format.md
└── tests/vectors/         # Golden Taproot descriptor fixtures
```

## Policy example

Full field reference: [docs/policy-format.md](docs/policy-format.md).

```json
{
  "version": 1,
  "network": "testnet",
  "script_type": "taproot",
  "keys": [
    { "id": "A", "role": "investor", "xpub": "xpub...", "fingerprint": "a1b2c3d4" },
    { "id": "B", "role": "manager",  "xpub": "xpub...", "fingerprint": "e5f6a7b8" },
    { "id": "C", "role": "recovery", "xpub": "xpub...", "fingerprint": "c9d0e1f2" }
  ],
  "policy": {
    "primary": "(A && B) || (A && C)",
    "fallback": { "after": "4y", "allow": "A" }
  }
}
```

Networks: `mainnet` | `testnet` (Testnet3) | `testnet4` | `signet` | `regtest`.

## License

Apache-2.0
