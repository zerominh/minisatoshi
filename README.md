# Minisatoshi

Offline-first desktop app for creating and managing Bitcoin vaults with Miniscript.

## Status

**v0.2.x** — Phases 1–4 (vaults, signing, import/export, interop docs).

See [CHANGELOG.md](CHANGELOG.md) and [docs/DEVELOPMENT_PLAN.md](docs/DEVELOPMENT_PLAN.md).

## Docs

| Doc | Topic |
|---|---|
| [docs/interop.md](docs/interop.md) | Sparrow / Liana / Nunchuk / Core — fund vs watch vs sign |
| [docs/bitcoin-core-miniscript.md](docs/bitcoin-core-miniscript.md) | `importdescriptors`, PSBT, multipath |
| [docs/hardware-signing.md](docs/hardware-signing.md) | HWI, BIP-388, Coldcard |
| [docs/policy-format.md](docs/policy-format.md) | Policy JSON schema |

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
│   ├── blockchain/        # Esplora / Electrum + fund/server presets
│   ├── psbt-engine/
│   ├── signing-devices/   # HWI
│   └── vault/
├── docs/
│   ├── DEVELOPMENT_PLAN.md
│   ├── interop.md
│   ├── bitcoin-core-miniscript.md
│   ├── hardware-signing.md
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
