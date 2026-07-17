# Hardware signing (Sprint 11 + Ledger ABC)

Minisatoshi is watch-only by default. Hardware wallets sign via **HWI** (Coldcard, Trezor, singlesig Ledger) and **`ledger-bitcoin`** (Ledger Miniscript Taproot script-path / ABC). Secrets never leave the device.

See also: [Ledger wallet policy plan](ledger-wallet-policy-plan.md) · [Interop matrix](interop.md) · [Bitcoin Core + Miniscript](bitcoin-core-miniscript.md)

## Supported path

1. Create / open a Taproot **wallet** (e.g. ABC `(A && B) || (A && C)`).
2. **Settings** → **Signing devices** — install/verify HWI; for Ledger ABC also **Install Ledger signer**.
3. **Wallet → Settings → Register on hardware** — BIP-388 policy; for Ledger ABC use **Register Ledger policy**.
4. **Send** — create PSBT → sign on each required device → combine → finalize → broadcast.

Primary spends for the default ABC template need **Investor (A) + Manager (B)**. Recovery / timelock paths use different key sets and usually need a BIP68 `input sequence`.

## Ledger + Miniscript Taproot (ABC)

ABC wallets spend via **Taproot script-path** (`tap_bip32_paths`), not the internal NUMS key.

| Stack | Role |
|-------|------|
| **HWI 3.2** | Enumerate, `getxpub`, Coldcard/Trezor `signtx`, singlesig Ledger |
| **ledger-bitcoin** (bundled venv) | Ledger `register_wallet` + script-path `sign_psbt` for ABC |

**Stock HWI 3.2 cannot sign ABC on Ledger** — its Ledger driver skips script-path keys ([HWI #827](https://github.com/bitcoin-core/HWI/issues/827)). Minisatoshi routes Ledger + ABC to `ledger-bitcoin` after wallet-policy registration.

### Ledger ABC workflow

1. **Settings** → **Install Ledger signer** — installs `ledger-bitcoin` 0.4.1 + USB HID (`ledgercomm[hid]`) into app data (`{data_dir}/ledger/venv/`). First install may need system Python once for bootstrap. Re-run if you see an hidapi error.
2. **Wallet → Settings** → **Register Ledger policy** — BIP-388 registration on device; app stores HMAC locally.
3. **Send** → **Sign with hardware** (same fingerprint) — `tap_script_sigs` added via ledger-bitcoin.

**Verify device** (USB only) ≠ policy registration. Ledger shows the policy only during **Register Ledger policy** or **Sign**.

### Registration staleness

Registration is bound to a **policy fingerprint** (BIP-388 template + keys) and **network**. If you change the descriptor, keys, or network, status shows **stale** — re-register on Ledger before signing.

### Common Ledger errors

| Symptom | What to do |
|---------|------------|
| “Register Ledger policy first” | Wallet → Settings → Register Ledger policy |
| “stale” / descriptor changed | Re-register after policy edits |
| “did not respond in time” | Unlock device, open Bitcoin app, approve (180s timeout) |
| “network mismatch” | Match wallet network and Bitcoin app (testnet vs mainnet) |
| “firmware / app too old” | Bitcoin app ≥ 2.1, update device firmware |
| User cancelled | Normal — retry when ready |

### Other workarounds

| Method | Notes |
|--------|-------|
| Software / hot cosigner | Sign one ABC key in Minisatoshi; hardware for the other |
| Coldcard | HWI USB or air-gapped SD PSBT |
| External signer | Export PSBT to Liana, Sparrow, etc. |

## BIP-388 / Ledger

Minisatoshi rewrites the wallet descriptor into a **wallet policy** template:

- Keys become `@0`, `@1`, … with key-info strings `[fingerprint/origin]xpub`.
- The Miniscript tree stays as in the output descriptor (`tr(NUMS,{…})`).

| Method | Notes |
|--------|-------|
| **Register Ledger policy** (ABC) | ledger-bitcoin `register_wallet` → HMAC in `{data_dir}/ledger_registrations/` |
| **Register on device** (non-ABC / HWI) | HWI `registerpolicy` when available, else `displayaddress` |
| **Save BIP-388 JSON** | Manual / other tools |
| **Newer HWI** | If `registerpolicy` exists, HWI path may return HMAC |

## HMAC storage (security)

- HMAC is a **proof of wallet-policy registration** (BIP-388), not a seed or private key.
- Stored as JSON under app data (`ledger_registrations/{wallet_id}/{fingerprint}.json`), alongside the local SQLite vault DB.
- **Not encrypted at rest** beyond OS user profile permissions — same trust model as other Minisatoshi local data.
- Each Ledger (fingerprint) has its own HMAC for the same vault policy.

## Coldcard

1. Wallet → Settings → **Save Coldcard MicroSD file**.
2. Copy onto a MicroSD card.
3. On Coldcard (Mk4+): Advanced → MicroSD → import / descriptor workflows.
4. Sign PSBTs via USB (HWI `signtx`) or air-gapped SD; **Combine** in Send.

## Trezor

HWI `signtx` works for many scripts. Miniscript + Taproot script-path support depends on firmware — **test on testnet** before mainnet.

## Multi-device cosign

Example ABC primary path:

1. Sign with Investor device (fingerprint of key A).
2. Sign with Manager device (fingerprint of key B) — same PSBT or **Combine** partial PSBTs.
3. Finalize → broadcast (Esplora).

Send lists wallet keys (id · fingerprint · role) as a checklist.

## Timelocks (`older` / BIP68)

Timelock paths need the correct **input `nSequence`**. Set **Input sequence** in Send when spending a fallback path.

## Networks

| Wallet network | HWI `--chain` | ledger-bitcoin `--chain` |
|----------------|---------------|--------------------------|
| mainnet | `main` | `main` |
| testnet / testnet3 | `test` | `test` |
| testnet4 | `testnet4` | `test` |
| signet | `signet` | `signet` |
| regtest | `regtest` | `regtest` |

Confirm network on device and in the app before registering or signing.

## Future: HWI upstream

Track [HWI #827](https://github.com/bitcoin-core/HWI/issues/827) (Taproot script-path on Ledger). When upstream supports ABC signing, Minisatoshi may route Ledger ABC back to a single HWI stack.

## Out of scope (today)

Jade / BitBox / Specter DIY → later. Prefer Ledger (ABC via ledger-bitcoin) + Coldcard for Miniscript Taproot.
