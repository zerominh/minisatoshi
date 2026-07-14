# Changelog

All notable changes to Minisatoshi are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.2] — 2026-07-14

### Fixed

- Sync no longer freezes the UI: Esplora HTTP runs off the wallet-store lock (and sync_vault on a worker thread)
- Esplora sync: ~250ms between calls, longer 429 backoff; empty wallet still needs ~40 address probes (gap 20 × receive+change) so public Blockstream can 429 on first open
- Hot wallet open after parent SQLite wallet was deleted (or BIP-86 migration recreated storage): heal stale `linked_wallet_id` instead of `wallet not found`
- Hot wallet rename still updates the keystore name when the linked vault row is missing
- Hot wallet singlesig used `tr(NUMS,{pk(A)})` script-path addresses, so Sync showed **0 balance** vs Sparrow BIP-86 `tr(xpub/…)`. Now compiles key-path Taproot; opening an old hot wallet migrates the stored descriptor.
- Hot wallet signing produced **invalid Taproot script-path signatures** for multipath descriptors (`[fp/origin]account/<0;1>/*`). Secrets now use `[fp]master/86'/…'/<0;1>/*`; existing keystores rebuild the path from the mnemonic on sign.
- Finalize errors now include the underlying Miniscript/interpreter reason (e.g. bad Schnorr signature).
- Software signing verifies tap_script_sigs after Sign so a wrong key fails immediately instead of at Finalize.

### Changed

- Send: Create PSBT → Sign → Broadcast as slides; success screen after broadcast (View transactions / Send another)

### Added

- Sticky flash banner (errors / success) fixed at the top of main content app-wide
- Auto chain sync when opening a vault / hot wallet (deferred, cache-aware), then every 2 min in background without locking UI
- Vault shell UI (Sparrow-style per-vault tabs): Transactions · Send · Receive · Addresses · UTXOs · Settings
- Hot wallets are first-class Bitcoin wallets in the UI (`/hot-wallets/:id` shell); vault storage is internal only
- Import no longer asks for a parent vault; open/list under Hot wallets
- `open_hot_wallet` / auto parent wallet when chain storage is missing
- Regression: BIP-86 receive addresses match BIP vectors; create → key-path sign → finalize
- `list_addresses` IPC for the Addresses tab
- Hot wallets: Sparrow-style numbered BIP-39 word grid (12/24) with autocomplete
- SeedQR import: camera or image scan (Standard + Compact SeedQR / plain words / JSON)

## [0.3.1] — 2026-07-14

### Added

- Delete wallet (cascades all nested vaults) and delete vault from list / vault detail (with confirm)

## [0.3.0] — 2026-07-14

### Added

- **Hot wallets (test):** BIP-39 / Sparrow-ish mnemonic JSON import; nested single-key Taproot vault under a parent wallet
- Encrypted hot keystore (`hot-keystore.v1`, Argon2id + XChaCha20-Poly1305 + master password) — SQLCipher deferred (cannot dual-link with plain SQLite)
- Send: **Sign with hot wallet** after unlocking keystore
- UI: `/hot-wallets`

## [0.2.3] — 2026-07-14

### Added

- **Interop docs:** `docs/interop.md` (Sparrow fund-only vs Core/Nunchuk/HW sign), `docs/bitcoin-core-miniscript.md` (`importdescriptors` / PSBT / multipath)
- README links for Phase 4 docs; Settings/Receive/Share copy audit (no Sparrow-to-sign)

## [0.2.2] — 2026-07-14

### Added

- **Watch-only share:** Vault → Share with chunked descriptor QR (`MSDESC1`), file + BSMS 1.0 export, instructions for third-party balance tracking
- **Import:** best-effort BSMS / Liana-ish JSON / multi-QR paste; BIP-380 checksum computed when missing
- **Watch-only badge** on vault list and detail (no private keys stored)

## [0.2.1] — 2026-07-14

### Added

- **Vault backup/restore:** `minisatoshi-vault-v1.json` export + import (descriptor + optional policy); checksum verify; network mismatch rejection
- UI: Vaults → Import vault; Vault → Export vault backup

## [0.2.0] — 2026-07-14

### Added

- **In-app signing (Phase 3):** software hot-key sign, combine, finalize, Esplora broadcast
- **HWI bridge:** device enumerate / getxpub / signtx; auto-install official HWI 3.2.0 (SHA-256 verified)
- **Hardware registration:** BIP-388 policy mapping, Coldcard MicroSD export, Ledger/Trezor guidance (`docs/hardware-signing.md`)
- **Send UX (Sprint 12):** spending-path picker (primary / timelock), signature status (“need A+B · have A · missing B”), network-confirmed broadcast, double confirm for mainnet hot keys
- Policy templates (ABC, 2-of-3, inheritance, dead man’s switch, multi-manager) and multi-fallback paths

### Changed

- Sparrow messaging: fund-only; do not claim arbitrary Miniscript import/sign support

### Security

- Mainnet software signing remains off by default (two checkboxes required)
- Secrets never logged; `tprv`/`xprv` redacted in UI errors
- Optional SQLCipher hot-key store deferred (HW-only path does not require it)

## [0.1.0] — 2026-07-14

### Added

- Offline-first Tauri desktop app (Windows / macOS / Linux) for Miniscript vaults
- Policy engine (JSON → Miniscript) with ABC investor/manager/recovery preset and timelock fallback
- Descriptor engine compiling Taproot (`tr`) descriptors with NUMS internal key + checksum
- SQLite storage and wallet/vault lifecycle (`wallet-core`, `vault`)
- Address derivation, Esplora/Electrum sync, Sparrow watch-only export, unsigned PSBT create/export
- Explicit **Testnet3** (`testnet`) and **Testnet4** (`testnet4`) network support
- Golden Taproot descriptor vectors and vault lifecycle integration test
- User-facing error sanitization (redacts `xprv` / `tprv` before IPC)
- Policy format docs and GitHub multi-OS release workflow

### Security

- App is watch-only: policies and storage accept xpubs only; private keys never leave hardware / Sparrow workflows in v0.1
