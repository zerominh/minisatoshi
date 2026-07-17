# Interop matrix

Minisatoshi stores **watch-only** vaults (xpubs + descriptors). Signing uses in-app software hot-keys (test/dev), **HWI hardware**, or an external Miniscript-capable wallet. This page states what each external tool can and cannot do.

Related:

- [Bitcoin Core + Miniscript](bitcoin-core-miniscript.md)
- [Hardware signing (HWI)](hardware-signing.md)
- [Policy format](policy-format.md)

## Quick answer

| Goal | Use |
|---|---|
| **Fund** a Minisatoshi receive address | Any wallet (Sparrow, Exchange, Core, …) — send to the address shown in **Receive** |
| **Watch** the full vault policy | Minisatoshi, Bitcoin Core, Nunchuk, Liana (Liana policies), descriptor import |
| **Sign** a Miniscript / Taproot script-path spend | Minisatoshi (HW / software), Bitcoin Core + HWI, Nunchuk — **not Sparrow** for arbitrary `tr(…)` vaults |

## Capability matrix

Legend: **Yes** / **Partial** / **No** / **N/A**

| Tool | Fund address | Import Miniscript vault (watch) | Sign Miniscript / tapscript PSBT | Notes |
|---|---|---|---|---|
| **Minisatoshi** | Yes (Receive) | Yes (Import / backup / BSMS) | Yes (HWI + optional hot-key) | Source of truth for policy compile + sync |
| **Sparrow** | Yes | **No** (arbitrary Miniscript `tr` script-path) | **No** | Keep Electrum/Esplora **server presets** only; fund by address |
| **Bitcoin Core** (≥ 26) | Yes | Yes (`importdescriptors`) | Yes (`walletprocesspsbt` + HWI) | Prefer multipath `<0;1>/*`; see Core guide |
| **Nunchuk** | Yes | Yes (descriptor / BSMS) | Yes (HW + companion) | Strong Miniscript / recovery UX |
| **Liana** | Yes | **Partial** | **Partial** | Restores **Liana** wallets; not a generic import of every Minisatoshi policy |
| **Ledger / Coldcard / Trezor** | N/A | Via registration (BIP-388 / MicroSD) | Yes (via HWI `signtx`) | Register before first co-sign — [hardware-signing.md](hardware-signing.md) |

## Sparrow (fund only)

Sparrow remains useful to **send coins to** a Minisatoshi receive address on the same network.

Do **not** expect:

- Importing a Minisatoshi Taproot script-path descriptor as a full wallet
- Signing that vault’s PSBT inside Sparrow

Settings → **Server presets** lists Electrum/Esplora URLs Sparrow users often share — that is for chain backends, not vault import.

## Recommended flows

### Fund → watch → spend (primary)

```text
Receive address (Minisatoshi)
        │  send from Sparrow / any wallet
        ▼
Sync in Minisatoshi (Esplora)
        │
        ▼
Send → PSBT → HW/software sign (or Core / Nunchuk) → finalize → broadcast
```

### Share watch-only with a third party

Wallet → **Share** (descriptor QR / file / BSMS) → they **Import** in Minisatoshi (or Core/Nunchuk). They never receive seeds.

### External Core signing

1. Export descriptor from Minisatoshi (Share / Receive).
2. Import into Core (`importdescriptors`) — [bitcoin-core-miniscript.md](bitcoin-core-miniscript.md).
3. Create PSBT in Minisatoshi **or** Core; process/sign with Core + HWI; combine back if needed.

## Formats Minisatoshi speaks

| Format | Role |
|---|---|
| Checksummed descriptor (`tr…#…` / `wsh…#…`) | Source of truth |
| `minisatoshi-vault-v1.json` | Backup + optional policy |
| BSMS 1.0 descriptor record | Watch-only share / Nunchuk-ish |
| PSBT (BIP-174 base64) | Create / sign / combine / finalize |
| BIP-388 + Coldcard MicroSD | HW registration |

## Honesty checklist (copy audit)

- Prefer “fund with Sparrow” over “import into Sparrow”.
- Prefer “sign in Minisatoshi / Core / Nunchuk / hardware” — never “paste into Sparrow to sign” for Miniscript vaults.
- Liana is **not** billed as a generic Miniscript open for every policy.
