# Kế hoạch: Ledger wallet policy song song HWI (Phương án 1)

Trạng thái: **Draft** · Cập nhật: 2026-07-17

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
  └─ ledger-bitcoin (NEW, Python subprocess → bundle sau)
        register_wallet(policy) → HMAC
        sign_psbt(psbt, policy, hmac) → tap_script_sigs
```

**Routing:**

| Điều kiện | Backend |
|-----------|---------|
| Ledger + ABC script-path + đã register (có HMAC) | `LedgerPolicyClient` |
| Ledger + ABC + chưa register | Lỗi rõ: "Register Ledger policy in Settings" |
| Coldcard / Trezor / singlesig | `HwiClient` (như hiện tại) |

## Phạm vi

### In scope (trung hạn)

- Register BIP-388 → Ledger → lưu HMAC per `(wallet_id, fingerprint)`
- Sign PSBT script-path qua `ledger-bitcoin`
- Router trong `hw_register_wallet` / `hw_sign_psbt`
- UI: Register Ledger policy, trạng thái registered
- Network: `--chain` khớp wallet (`main`, `test`, …)

### Out of scope (giai đoạn đầu)

- Rust binding trực tiếp `ledger_bitcoin` (sau POC Python)
- Trezor/Coldcard qua ledger path
- Chờ HWI upstream thay thế hoàn toàn
- MuSig2 nâng cao

## Công nghệ

| Thành phần | Ghi chú |
|------------|---------|
| [`ledger-bitcoin`](https://pypi.org/project/ledger-bitcoin/) ~0.4.x | `WalletPolicy`, `register_wallet`, `sign_psbt` |
| BIP-388 package | Đã có trong `crates/signing-devices/src/registration.rs` |
| Python 3 | POC: system Python; production: embed hoặc PyInstaller |

### API Python (tham chiếu)

```python
from ledger_bitcoin import createClient, Chain, WalletPolicy

client = createClient(chain=Chain.TEST)
policy = WalletPolicy(name, descriptor_template, keys)
hmac = client.register_wallet(policy)
signed_psbt = client.sign_psbt(psbt, policy, hmac)
```

## Lưu trữ app data

```
{data_dir}/
  hwi/3.2.0/hwi.exe
  ledger/
    ledger_cli.py
    venv/                    # hoặc embed python + wheels
  ledger_registrations/
    {wallet_id}/
      {fingerprint}.json
```

### Schema `LedgerRegistration`

```json
{
  "walletId": "uuid",
  "fingerprint": "a98a1256",
  "network": "testnet",
  "policyName": "ABC Vault",
  "policyTemplate": "tr(NUMS,{{and_v(v:pk(@0/**),pk(@1/**)),...}})",
  "keys": ["[fp/path]xpub", "..."],
  "hmacHex": "…",
  "registeredAt": "2026-07-17T…"
}
```

- HMAC = proof of registration (BIP-388), không phải seed; lưu local.
- Invalidate khi descriptor / policy wallet thay đổi.

## Module Rust mới

```
crates/signing-devices/src/
  ledger/
    mod.rs       # LedgerPolicyClient
    cli.rs       # subprocess ledger_cli.py, JSON protocol
    store.rs     # load/save registrations
    policy.rs    # Bip388Policy → JSON cho Python
  hwi.rs         # unchanged
  registration.rs  # + to_ledger_wallet_policy()
```

### API Rust (draft)

```rust
pub struct LedgerPolicyClient { /* python + script paths, chain */ }

impl LedgerPolicyClient {
    pub fn register(&self, fingerprint: &str, policy: &Bip388Policy) -> Result<String, SignError>;
    pub fn sign_psbt(&self, fingerprint: &str, psbt_b64: &str, reg: &LedgerRegistration) -> Result<String, SignError>;
}
```

### Python CLI protocol

```bash
ledger_cli.py register --chain test
# stdin:  {"fingerprint":"…","name":"…","policy":"…","keys":[…]}
# stdout: {"ok":true,"hmac":"hex"} | {"error":"…"}

ledger_cli.py sign --chain test
# stdin:  {"fingerprint":"…","psbt":"base64…","name":"…","policy":"…","keys":[…],"hmac":"hex"}
# stdout: {"ok":true,"psbt":"base64…"} | {"error":"…"}
```

## Tauri commands & UI

### Backend

| Command | Thay đổi |
|---------|----------|
| `hw_register_wallet` | Ledger + ABC → `LedgerPolicyClient::register` + lưu HMAC |
| `hw_sign_psbt` | Router Ledger vs HWI; gỡ block `HWI_LEDGER_SCRIPT_PATH_MSG` khi Ledger path OK |
| `get_ledger_registration_status` | **NEW** |
| `ensure_ledger_runtime_installed` | **NEW** (Phase 3) |

### Frontend

- **Wallet → Settings:** nút **Register Ledger policy**; badge Registered / Not registered
- **Send:** disable Ledger sign nếu ABC chưa register; checklist cosigner A/B
- Phân biệt **Verify USB** vs **Register policy** (có prompt trên device)

### Docs liên quan

- Cập nhật `docs/hardware-signing.md` khi Phase 2 xong
- Thêm mục vào `docs/DEVELOPMENT_PLAN.md` (sprint Ledger ABC)

## Phases

### Phase 0 — POC thủ công (3–5 ngày)

**Mục tiêu:** 1 PSBT ABC testnet ký được trên Ledger (ít nhất 1 cosigner).

- [ ] `tools/ledger_cli.py` + `pip install ledger-bitcoin`
- [ ] BIP-388 từ `prepare_hw_registration` / fixture ABC testnet
- [ ] `register` → HMAC → `sign` → kiểm tra `tap_script_sigs`
- [ ] Ghi firmware Ledger, policy string chính xác, lỗi thường gặp

**Done:** PSBT có chữ ký script-path từ 1 Ledger.

---

### Phase 1 — Rust + storage (1–2 tuần)

- [ ] `ledger/store.rs`, `ledger/cli.rs`, `LedgerPolicyClient`
- [ ] `to_ledger_wallet_policy()` từ `RegistrationPackage`
- [ ] `hw_register_wallet` → register thật + lưu HMAC
- [ ] `get_ledger_registration_status`
- [ ] Unit tests: policy mapping, JSON; mock CLI integration test

**Done:** Register + sign qua Tauri trên máy dev có Ledger.

---

### Phase 2 — Sign router + UX (1 tuần)

- [ ] `hw_sign_psbt` routing
- [ ] UI Settings + Send
- [ ] Flow 2 cosigner: sign A → sign B (cùng PSBT hoặc Combine)

**Done:** ABC testnet E2E: register → sign A+B → finalize → broadcast.

---

### Phase 3 — Bundle runtime (1–2 tuần)

- [ ] `ensure_ledger_runtime_installed` (embed Python hoặc PyInstaller)
- [ ] Checksum, Windows / macOS / Linux
- [ ] Settings → Install Ledger signer

**Done:** User không cần Python system.

---

### Phase 4 — Hardening

- [ ] Error mapping (reject, timeout, wrong network, firmware cũ)
- [ ] Re-register khi đổi descriptor
- [ ] Review bảo mật HMAC storage
- [ ] Theo dõi HWI #785, #827 — gộp lại một đường HWI khi upstream đủ

## Tiêu chí thành công

1. Register: user duyệt policy trên Ledger; app lưu HMAC.
2. Sign: `hw_sign_made_progress` = true; `tap_script_sigs` tăng.
3. E2E testnet ABC primary (A+B) finalize + broadcast.
4. HWI không regress (Coldcard vẫn qua HWI).
5. Không còn message gây hiểu nhầm "register Settings = ký được" khi chưa register Ledger policy.

## Rủi ro

| Rủi ro | Giảm thiểu |
|--------|------------|
| Policy Ledger reject | Phase 0 validate; so với Liana / BIP-388 ref |
| Python trên Windows | POC trước; PyInstaller / embed |
| Firmware Ledger cũ | Check version; message rõ (Bitcoin app ≥ 2.1) |
| 2 Ledger, 1 policy | Register từng fingerprint; mỗi device một HMAC |
| ABC cần A+B | UI checklist; không claim 1 Ledger đủ |

## Quyết định cần chốt

1. POC dùng system Python (dev) — OK?
2. Mỗi Ledger (fingerprint) register riêng trên cùng wallet policy?
3. Testnet trước, mainnet sau?

## Thứ tự thực hiện

```
Phase 0 (POC script)
  → Phase 1 (Rust + store + register)
  → Phase 2 (sign router + UI)
  → Phase 3 (bundle)
```

Không làm UI production trước khi Phase 0 chứng minh Ledger ký được ABC.

## Tham chiếu

- [HWI 3.2.0 release](https://github.com/bitcoin-core/HWI/releases/tag/3.2.0)
- [HWI issue #827 — Taproot script-path](https://github.com/bitcoin-core/HWI/issues/827)
- [ledger-bitcoin PyPI](https://pypi.org/project/ledger-bitcoin/)
- [Ledger wallet policy doc](https://github.com/LedgerHQ/app-bitcoin-new/blob/master/doc/wallet.md)
- [BIP-388](https://github.com/bitcoin/bips/blob/master/bip-0388.mediawiki)
- Minisatoshi: `docs/hardware-signing.md`, `crates/signing-devices/src/registration.rs`
