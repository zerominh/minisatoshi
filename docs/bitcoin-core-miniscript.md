# Bitcoin Core + Miniscript vaults

Use this when you want Bitcoin Core (and HWI) to **watch** and/or **sign** a Minisatoshi vault.

Also read: [Interop matrix](interop.md) · [Hardware signing](hardware-signing.md)

## Requirements

| Item | Recommendation |
|---|---|
| Bitcoin Core | **≥ 26.0** (multipath descriptors `<0;1>/*`; older versions may need separate receive/change imports) |
| Wallet type | Descriptor wallet (`descriptors=true` — default for new wallets in recent Core) |
| Hardware | HWI + supported device if you sign without bringing `xprv` into Core |
| Network | Match Minisatoshi wallet network (`main` / `test` / `testnet4` / `signet` / `regtest`) |

Minisatoshi Taproot policies often use a NUMS internal key + script tree. Core must accept the descriptor and checksum as exported from Minisatoshi.

## 1. Export the descriptor

In Minisatoshi:

- Wallet → **Share** / **Receive** → copy descriptor, or
- Export `minisatoshi-vault-v1.json` / BSMS and take the descriptor line.

Expected shape (example; yours will differ):

```text
tr([fingerprint/…]xpub…/<0;1>/*,{…})#checksum
```

Keep the `#checksum` (BIP-380). If a tool stripped it, recompute with:

```bash
bitcoin-cli getdescriptorinfo "tr(…)"
```

Use the `"descriptor"` field that includes the checksum.

## 2. Import watch-only (importdescriptors)

Create or open a descriptor wallet, then:

```bash
bitcoin-cli -rpcwallet=minivault importdescriptors '[
  {
    "desc": "tr(…)#checksum",
    "timestamp": "now",
    "active": true,
    "range": [0, 1000],
    "internal": false
  }
]'
```

Notes:

- Multipath `<0;1>/*` covers receive (`/0/*`) and change (`/1/*`) in one descriptor on Core ≥ 26.
- If your export is receive-only (`/0/*`), import a second descriptor for change (`/1/*`) with `"internal": true`.
- `"timestamp": 0` (or a known creation height) forces a full rescan when you need historical balances.

Verify addresses match Minisatoshi index 0 before funding further:

```bash
bitcoin-cli -rpcwallet=minivault getnewaddress
# Compare with Minisatoshi Receive → index 0 (or Share / BSMS first address)
```

## 3. Sign with Core (walletprocesspsbt)

### Option A — soft keys inside Core (dev / recovery)

Only if you intentionally imported `xprv` / private descriptor material (not the default Minisatoshi export).

```bash
bitcoin-cli -rpcwallet=minivault walletprocesspsbt "cHNidP8…"
```

### Option B — hardware via HWI (recommended)

1. Export unsigned PSBT from Minisatoshi **Send** (base64).
2. Sign on device with HWI (`hwi signtx`) or Core’s hardware wallet tooling when configured.
3. Bring the partially/fully signed PSBT back to Minisatoshi (**Combine** / **Finalize**) or finalize in Core:

```bash
bitcoin-cli -rpcwallet=minivault finalizepsbt "cHNidP8…"
bitcoin-cli sendrawtransaction "02000000…"
```

Minisatoshi can also finalize and broadcast via its Esplora backend after combine.

Multi-sig / multi-key policies (e.g. ABC primary `A && B`):

- Each required fingerprint must sign.
- Use Minisatoshi’s signature status (“need A+B · have A · missing B”) or Core’s PSBT analysis between rounds.

## 4. Timelocks

Policies with `older(N)` / BIP68 need the correct input `nSequence`. Minisatoshi **Send** exposes **Input sequence** when you pick a timelock path. If you build the PSBT only in Core, set the sequence to match the leaf you intend to spend.

## 5. What Core is not

- A replacement for Coldcard MicroSD / BIP-388 registration UX — use **Wallet → Settings → Register on hardware** for those exports.
- Automatic understanding of Minisatoshi policy JSON — Core only sees the **compiled descriptor**.

## Troubleshooting

| Symptom | Likely cause |
|---|---|
| `importdescriptors` rejects checksum | Truncated paste; re-run `getdescriptorinfo` |
| Address ≠ Minisatoshi index 0 | Wrong network, wrong receive/change path, or non-multipath import |
| Device won’t sign | Wallet not registered / wrong chain flag / unsupported Miniscript fragment on firmware |
| Finalize fails | Missing cosigner signature or wrong timelock sequence |

When unsure, fund and sign on **testnet/signet** first.
