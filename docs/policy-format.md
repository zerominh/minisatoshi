# Minisatoshi Policy Format

Schema version: **1** (`POLICY_SCHEMA_VERSION`).

Policies are JSON documents that describe who can spend from a vault, under which conditions. Minisatoshi validates the document, compiles it to Miniscript, then to a Taproot (`tr`) or wrapped SegWit (`wsh`) output descriptor.

## Top-level fields

| Field | Type | Description |
|---|---|---|
| `version` | `u32` | Must be `1` |
| `network` | string | `mainnet` \| `testnet` (testnet3) \| `testnet4` \| `signet` \| `regtest` |
| `script_type` | string | `taproot` (default path) or `wsh` |
| `keys` | array | Key participants (see below) |
| `policy` | object | Spending rules |

## Keys

Each entry in `keys`:

| Field | Type | Description |
|---|---|---|
| `id` | string | Identifier used in expressions (`A`, `B`, `investor`, …) |
| `role` | string | `investor` \| `manager` \| `recovery` \| `cosigner` \| `other` |
| `xpub` | string | Extended public key (`xpub` / `tpub`) |
| `fingerprint` | string | 8-hex BIP32 master fingerprint |
| `origin_path` | string? | Account path without master `m/`, e.g. `86'/1'/0'`. A leading `m/` / `M/` is stripped automatically. |

Watch-only only: never put `xprv` / `tprv` in a policy document.

## Policy expression

```json
"policy": {
  "primary": "(A && B) || (A && C)",
  "fallback": { "after": "4y", "allow": "A" }
}
```

### Expression grammar

- Key refs: identifiers matching `[A-Za-z0-9_-]+`
- Operators: `&&` (and), `||` (or)
- Grouping: `( … )`
- Precedence: `&&` binds tighter than `||`

Examples:

- `A && B` — both required
- `(A && B) || (A && C)` — investor+manager or investor+recovery
- `(A && B) || (A && C) || (B && C)` — any 2-of-3

Every key referenced in `primary` / `fallback.allow` must exist in `keys`.

### Timelock fallback

| Field | Description |
|---|---|
| `after` | Relative timelock duration |
| `allow` | Key id(s) expression for the delayed path |

Duration forms:

| Input | Meaning |
|---|---|
| `1d` | 1 × 144 blocks (~1 day) |
| `2d` | 2 × 144 blocks |
| `1w` | 1 × 1 008 blocks (~1 week) |
| `4y` | 4 × 52 560 blocks (~4 years) |
| `210240b` | Explicit block count |
| `1008` | Plain block count |

Constants (≈10‑minute blocks):

- `BLOCKS_PER_DAY = 144`
- `BLOCKS_PER_WEEK = 1008`
- `BLOCKS_PER_YEAR = 52560` (365.25 × 144)

Fallback compiles to a Miniscript leaf like `and(pk(A), older(N))` alongside the primary leaves.

## Compilation result

For Taproot vaults, Minisatoshi:

1. Compiles each policy branch to a separate leaf (required for multi-key trees)
2. Wraps them in `tr(NUMS, {leaves…})` with an unspendable internal key
3. Appends a BIP380 descriptor checksum (`#…`)

Addresses are derived at `…/<0;1>/*` (receive / change).

## ABC preset

The built-in investor / manager / recovery template:

- Primary: `(A && B) || (A && C)`
- Fallback: investor `A` after `N` years
- Script: Taproot

Golden vectors live under [`tests/vectors/`](../tests/vectors/).

## Example

See [README](../README.md#policy-example) and `tests/vectors/policy_abc_testnet.json`.
