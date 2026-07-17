# Kế hoạch: Ledger wallet policy song song HWI (Phương án 1)

Trạng thái: **Implemented (Phases 0–4)** · Cập nhật: 2026-07-17

## Blocker notes (hardware, 2026-07-17)

- Probe OK: `Bitcoin Test` **2.4.6**
- Fixture-only keys (không khớp fingerprint thiết bị) → `0x6a80` / `0x6a82` — **không** kết luận được client sai
- **Không** cần DMK/npm — cùng APDU `REGISTER_WALLET`; `ledger-bitcoin` 0.4.1 đúng stack

**Baseline OK** trên thiết bị thật (`a98a1256`, Bitcoin Test 2.4.6):
`tools/ledger_register_baseline.cmd` → HMAC trả về khi policy có key của device.

Bài học: fixture không chứa fingerprint Ledger → `0x6a80`/`0x6a82` (nhiễu). UI register phải dùng key thật trong vault.

Tiếp: `tools/ledger_register_abc_smoke.cmd` (ABC taproot + device key A).  
`older(210240)` vẫn không tương thích app ≥ 2.4.6 (BIP68 max 65535).

## Bối cảnh

- Ví **ABC Miniscript Taproot script-path** (`tr(NUMS, {{and_v(...), ...}})`) spend qua `tap_bip32_paths`, không qua internal key.
- **HWI 3.2.0** (release mới nhất, Minisatoshi đã pin) **không** ký script-path trên Ledger (`hwilib/devices/ledger.py`: `TODO: Support script path signing`).
- **Ledger Bitcoin App ≥ 2.1** hỗ trợ Miniscript + Wallet Policy (BIP-388), nhưng cần stack gọi **`ledger-bitcoin`**, không chỉ `hwi signtx`.

## Mục tiêu

Ledger ký được ABC trong Minisatoshi, **không thay** HWI cho Coldcard / Trezor / enumerate / getxpub.

## Kiến trúc

```
Minisatoshi Desktop (Tauri)
  │
  ├─ HWI 3.2 (existing)
  │     enumerate, getxpub, Coldcard/Trezor signtx, singlesig
  │
  └─ ledger-bitcoin (Python subprocess, bundled venv)
        register_wallet(policy) → HMAC
        sign_psbt(psbt, policy, hmac) → tap_script_sigs
```

**Routing:**

| Điều kiện | Backend |
|-----------|---------|
| Ledger + ABC script-path + đã register (HMAC hợp lệ, không stale) | `ledger-bitcoin` |
| Ledger + ABC + chưa register / stale | Lỗi rõ + UI badge |
| Coldcard / Trezor / singlesig | `HwiClient` (như hiện tại) |

## Lưu trữ app data

```
{data_dir}/
  hwi/3.2.0/hwi.exe
  ledger/
    ledger_cli.py
    venv/                    # ledger-bitcoin 0.4.1
    runtime.json
  ledger_registrations/
    {wallet_id}/
      {fingerprint}.json
```

### Schema `LedgerRegistration`

```json
{
  "walletId": "uuid",
  "fingerprint": "a98a1256",
  "hmac": "…",
  "policyFingerprint": "sha256hex",
  "network": "testnet",
  "registeredAtSecs": 1710000000
}
```

- HMAC = proof of registration (BIP-388), không phải seed.
- `policyFingerprint` = SHA-256 của BIP-388 template + keys; đổi descriptor → **stale** → re-register.
- Không mã hóa at-rest (cùng trust model với SQLite vault).

## Phases

### Phase 0 — POC thủ công ✅

- [x] `tools/ledger_cli.py` + `ledger-bitcoin`
- [x] BIP-388 từ `prepare_hw_registration`
- [x] `register` → HMAC → `sign` → `tap_script_sigs`

### Phase 1 — Rust + storage ✅

- [x] `ledger/store.rs`, `ledger/cli.rs`
- [x] `hw_register_wallet` → register + lưu HMAC
- [x] `get_ledger_registration_status`

### Phase 2 — Sign router + UX ✅

- [x] `hw_sign_psbt` routing Ledger vs HWI
- [x] UI Settings (wallet) — Register Ledger policy, badge registered

### Phase 3 — Bundle runtime ✅

- [x] `ensure_ledger_runtime_installed` — venv + pip pin 0.4.1
- [x] Settings → Install Ledger signer
- [x] `get_ledger_runtime_status`

**Giới hạn:** lần cài đầu có thể cần Python hệ thống để bootstrap venv; chưa embed Python độc lập.

### Phase 4 — Hardening ✅

- [x] `map_ledger_cli_error` — cancel, timeout (180s), network, firmware, HMAC, disconnect
- [x] `policy_fingerprint` + `registration_stale_reason` — re-register khi đổi descriptor/keys/network
- [x] UI badge **stale** + message
- [x] Docs: `hardware-signing.md`, plan này
- [ ] Theo dõi [HWI #827](https://github.com/bitcoin-core/HWI/issues/827) — gộp lại HWI khi upstream đủ (ongoing)

## Tiêu chí thành công

1. Register: user duyệt policy trên Ledger; app lưu HMAC + fingerprint.
2. Sign: `hw_sign_made_progress` = true; `tap_script_sigs` tăng.
3. E2E testnet ABC primary (A+B) finalize + broadcast.
4. HWI không regress (Coldcard vẫn qua HWI).
5. Stale registration detected khi policy đổi.

## Tham chiếu

- [HWI 3.2.0 release](https://github.com/bitcoin-core/HWI/releases/tag/3.2.0)
- [HWI issue #827 — Taproot script-path](https://github.com/bitcoin-core/HWI/issues/827)
- [ledger-bitcoin PyPI](https://pypi.org/project/ledger-bitcoin/)
- [Ledger wallet policy doc](https://github.com/LedgerHQ/app-bitcoin-new/blob/master/doc/wallet.md)
- [BIP-388](https://github.com/bitcoin/bips/blob/master/bip-0388.mediawiki)
- Minisatoshi: `docs/hardware-signing.md`, `crates/signing-devices/src/registration.rs`
