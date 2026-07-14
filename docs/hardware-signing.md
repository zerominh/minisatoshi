# Hardware signing (Sprint 11)

Minisatoshi is watch-only by default. Hardware wallets sign via **HWI**; secrets never leave the device.

See also: [Interop matrix](interop.md) · [Bitcoin Core + Miniscript](bitcoin-core-miniscript.md)

## Supported path

1. Create / open a Taproot vault (e.g. ABC `(A && B) || (A && C)`).
2. **Settings → Signing devices** — install/verify HWI, connect Ledger / Coldcard / Trezor, note fingerprint.
3. **Vault → Register on hardware** — build BIP-388 policy + Coldcard MicroSD text; optionally try on-device register.
4. **Send** — create PSBT → sign on each required device → combine → finalize → broadcast.

Primary spends for the default ABC template need **Investor (A) + Manager (B)**. Recovery / timelock paths use different key sets and usually need a BIP68 `input sequence`.

## BIP-388 / Ledger

Minisatoshi rewrites the vault descriptor into a **wallet policy** template:

- Keys become `@0`, `@1`, … with key-info strings `[fingerprint/origin]xpub`.
- The internal key / Miniscript tree stays as in the output descriptor (`tr(NUMS,{…})`).

**Stock HWI 3.2.0** (auto-installed by the app) does **not** expose `registerpolicy`. Registration options:

| Method | Notes |
|---|---|
| First co-sign | Ledger Bitcoin app may prompt “Register account” when signing a policy PSBT |
| Export BIP-388 JSON | Vault → Save BIP-388 JSON — use with tools / Core builds that support wallet policies |
| Newer HWI builds | If `registerpolicy` exists, **Register on device** stores any returned HMAC |

Confirm receive addresses on-device whenever the firmware allows (`displayaddress` / wallet UI).

## Coldcard

1. Vault → **Save Coldcard MicroSD file**.
2. Copy onto a MicroSD card.
3. On Coldcard (Mk4+): Advanced → MicroSD → import / descriptor workflows (exact menus vary by firmware).
4. Sign PSBTs via USB (HWI `signtx`) or air-gapped SD card; paste the signed PSBT back into **Send → Combine**.

Coldcard is strong for Taproot script-path / multipath descriptors; always verify addresses and policy text on the device screen.

## Trezor

HWI `signtx` works for many scripts. Miniscript + Taproot script-path support depends on firmware — **test on testnet** before mainnet. Prefer the same BIP-388 export when your tool chain accepts it.

## Multi-device cosign

Example ABC primary path:

1. Sign with Investor device (fingerprint of key A).
2. Sign with Manager device (fingerprint of key B) — either on the partially-signed PSBT, or sign separately and **Combine** in the app.
3. Finalize → broadcast (Esplora).

The Send screen lists vault keys (id · fingerprint · role) as a checklist.

## Timelocks (`older` / BIP68)

Inheritance / dead-man’s-switch leaves need the correct **input `nSequence`**. In Send, set **Input sequence** when spending a timelock path. Signing the wrong leaf (or too early) will fail on-device or at finalize.

## Networks

HWI `--chain` follows the vault network (`main`, `test`, `testnet4`, `signet`, `regtest`). Always confirm you are on the intended network before registering or signing.

## Out of Sprint 11 scope

Jade / BitBox / Specter DIY → later. Prefer Ledger + Coldcard (or Trezor) for Miniscript Taproot vaults today.
