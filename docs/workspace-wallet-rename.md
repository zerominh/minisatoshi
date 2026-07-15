# Rename: Wallet → Workspace, Vault → Wallet (phương án A)

> Trạng thái: **đã implement** (schema v2 + IPC + UI).  
> Mục tiêu: khớp ngôn ngữ thị trường Bitcoin — cái cầm tiền gọi là **Wallet / Ví**; container theo network gọi là **Workspace** (nội bộ).
>
> **UX (2026-07):** Workspace **ẩn khỏi primary UI**. Người dùng chọn **network** khi tạo/import ví; app `ensureWorkspaceForNetwork` get-or-create. Nav mặc định → `/wallets`. Route `/workspaces` vẫn có (advanced), không nằm trong sidebar.

**Đã xong trên cây code:**
- SQLite `SCHEMA_VERSION = 2` (`workspaces` + `wallets`), migrate từ v1
- Domain Rust (`storage`, `wallet-core`, `vault` → `WalletService`, …)
- Tauri IPC rename (xem bảng Phase 3)
- Hot keystore `linked_workspace_id` / `linked_wallet_id` + migrate-on-read
- Frontend routes `/workspaces`, `/wallets`, redirect `/vaults/*`
- Backup export `minisatoshi-wallet-v1`; import vẫn đọc `minisatoshi-vault-v1`

Còn tùy chọn sau: rename folder crates (`wallet-core` / `vault`); scrub hoàn toàn chữ “vault” trong docs cũ.

---

## 1. Vì sao đổi

Hôm nay trong app:

| Tên hiện tại | Thực chất |
| --- | --- |
| **Wallet** | Thùng chứa + network (không có số dư / descriptor) |
| **Vault** | Đơn vị spendable (policy / descriptor / UTXO / gửi-nhận) |

Trên thị trường, **wallet** gần như luôn = thứ người dùng mở để xem số dư và chi tiêu. Đặt container là “wallet” dễ khiến người mới tưởng tạo xong là dùng được, rồi bối rối vì còn phải “tạo két”.

**Phương án A (đã chọn):** đổi tên cho khớp thị trường, **và** rename API/DB (không chỉ label UI).

---

## 2. Bảng tên đích

| Hiện tại (code / route / UI) | Sau rename | UI EN | UI VI | Vai trò |
| --- | --- | --- | --- | --- |
| `Wallet`, `/wallets` | **Workspace** | Workspace | Không gian | Container + network |
| `Vault`, `/vaults` | **Wallet** | Wallet | Ví | Descriptor, balance, send/receive |
| Hot wallet | **Hot wallet** (giữ) | Hot wallet | Ví nóng | Keystore seed → trỏ một Wallet (ex-Vault) |
| Multi-key Miniscript vault | cùng entity Wallet + badge/loại | Multisig wallet | Ví đa chữ ký | Policy Miniscript |

**Câu onboarding một dòng**

> Chọn mạng → tạo ví (đa chữ ký hoặc nóng). Workspace theo network được tạo ngầm.

### Không đổi trong đợt này

- Semantics hot (vẫn một wallet spendable phía sau).
- Policy Miniscript / descriptor / HWI flow.
- Đổi tên folder crates (`wallet-core`, `vault`) — có thể làm sau; ưu tiên rename **type / table / IPC**.

---

## 3. Mô hình dữ liệu sau đổi

```
workspaces 1───* wallets 1───* addresses
                      └───* transactions

hot-keystore record:
  linked_workspace_id  → Workspace (parent)
  linked_wallet_id     → Wallet (spendable, ex-vault)
```

### Cột / field cần đổi nghĩa (nguy hiểm)

Hot keystore hiện có:

| Field cũ | Nghĩa cũ | Field mới | Nghĩa mới |
| --- | --- | --- | --- |
| `linked_wallet_id` | Parent container (ex-Wallet) | `linked_workspace_id` | Cùng nghĩa parent |
| `linked_vault_id` | Spendable (ex-Vault) | `linked_wallet_id` | Spendable |

**Bắt buộc** migration + test: nếu chỉ rename chữ mà không map nghĩa sẽ tráo hai ID.

SQLite (hiện `SCHEMA_VERSION = 1` trong `crates/storage/src/schema.rs`):

| Bảng / cột cũ | Mới |
| --- | --- |
| `wallets` | `workspaces` |
| `vaults` | `wallets` |
| `vaults.wallet_id` | `wallets.workspace_id` |
| `addresses.vault_id` | `addresses.wallet_id` |
| `transactions.vault_id` | `transactions.wallet_id` |
| indexes `idx_*_vault_*` | `idx_*_wallet_*` / `idx_wallets_workspace_id` |

---

## 4. API / IPC (Tauri)

Hướng: **breaking rename** (app desktop nội bộ). Không giữ dual-command lâu trừ khi cần tạm trong PR.

| Command / DTO cũ | Mới |
| --- | --- |
| `create_wallet` (container) | `create_workspace` |
| `list_wallets` | `list_workspaces` |
| `delete_wallet` / `rename_wallet` (container) | `delete_workspace` / `rename_workspace` |
| `create_vault` | `create_wallet` |
| `list_vaults` | `list_wallets` |
| `get_vault`, `delete_vault`, `rename_vault` | `get_wallet`, `delete_wallet`, `rename_wallet` |
| `sync_vault`, nhận địa chỉ / export… theo `vault_id` | `sync_wallet`, `wallet_id`, … |
| `WalletDto` (container) | `WorkspaceDto` |
| `VaultDto` | `WalletDto` (spendable) |
| fields `walletId` trên vault | `workspaceId` |

Cập nhật `commands.rs`, `dto.rs`, `lib.rs`, permissions `app.toml`.

Domain Rust (`wallet-core`, service hiện tên `VaultService`):

| Type / API cũ | Mới |
| --- | --- |
| `Wallet` (container) | `Workspace` |
| `Vault` | `Wallet` |
| `VaultService` | `WalletService` (crate folder `vault` có thể đổi sau) |
| `EmptyWalletName` (container) | `EmptyWorkspaceName` |
| lỗi empty vault name | `EmptyWalletName` |

---

## 5. Frontend

### Routes

| Cũ | Mới |
| --- | --- |
| `/wallets` | `/workspaces` |
| `/vaults`, `/vaults/:id/…` | `/wallets`, `/wallets/:id/…` |
| `/hot-wallets` | giữ |

Thêm redirect từ path cũ → mới (bookmark / deep link).

### Code / i18n / storage local

- Context: `VaultContext` → provider theo **Wallet** đang mở (có thể `WalletShellContext`).
- Pages: `WorkspacesPage`, list ví = ex-`VaultsPage`, `NewWalletPage` = ex-`NewVaultPage`, …
- i18n: `nav.workspaces`, `nav.wallets`; bỏ copy “Két = container”.
- `localStorage`: `minisatoshi.activeWalletId` → `activeWorkspaceId` (+ đọc fallback key cũ một thời gian).

---

## 6. Backup / share

- Export mới: ví dụ format `minisatoshi-wallet-v1` (hoặc giữ JSON shape, đổi `format` string + field names nếu có).
- Import: **đọc được** `minisatoshi-vault-v1` (alias compat).
- Export chỉ format mới.
- Cập nhật `docs/interop.md`, Share / QR copy.

---

## 7. Phased implementation

### Phase 0 — Chuẩn bị

- Inventory grep: `vault`, `create_wallet`, backup format, tests.
- Chốt: IPC breaking; backup read-compat; không rename crate folders trong PR đầu.

### Phase 1 — SQLite

- Runner migrate theo `schema_version` (hiện essentially chỉ seed v1).
- `MIGRATION_V2`: rename tables/columns như bảng mục 3.
- Fresh DB = schema mới trực tiếp.
- Tests cascade delete workspace → wallets → addresses.

### Phase 2 — Domain Rust

- Rename types/store/service methods + tests (`vault_lifecycle`, storage, psbt fixtures `workspace_id`).

### Phase 3 — Tauri

- DTO + commands + permissions; `cargo check` / desktop build.

### Phase 4 — Hot keystore

- Field mới + **migrate-on-read** từ `linked_vault_id` / old `linked_wallet_id`.
- Giữ heal stale parent (workspace bị xóa).

### Phase 5 — Frontend

- `api.ts`, routes, context, pages, i18n, redirects, active workspace key.

### Phase 6 — Backup + docs + CHANGELOG

- Compat import v1; CHANGELOG breaking note; chỉnh `DEVELOPMENT_PLAN` / README chỗ terminology.

### Phase 7 — Verify

- `cargo test`, `tsc`, smoke: tạo workspace → tạo wallet (multisig) → import hot → sync / send / share.

---

## 8. Rủi ro và giảm thiểu

| Rủi ro | Giảm thiểu |
| --- | --- |
| Tráo `linked_workspace_id` / `linked_wallet_id` | Test golden + migrate-on-read có assert |
| Schema migrate nửa vời | Một `SCHEMA_VERSION = 2`; không ship app giữa chừng |
| Sót IPC / TS invoke name | Compile + grep; checklist commands |
| User DB cũ | Bắt buộc upgrade path khi open store |
| Đụng tên “Wallet” hai nghĩa trong cùng PR | Diff review chú ý parent vs spendable |

---

## 9. Tiêu chí hoàn thành

- [x] DB v2: bảng `workspaces` + `wallets` (spendable); không còn bảng `vaults`.
- [x] UI: Không gian / Ví / Ví nóng đúng nghĩa thị trường.
- [x] IPC: không còn command `*_vault*` (trừ alias deprecate tạm nếu cần).
- [x] Mở DB app cũ được; backup `minisatoshi-vault-v1` vẫn import.
- [x] Tests xanh + smoke đường chính (`cargo check` desktop, `tsc`, crate tests).

---

## 10. Ghi chú triển khai

- Implement theo phase 1 → 7; không bắt đầu UI trước khi storage + domain ổn.
- Có thể tách PR: (1) storage+domain+tauri, (2) frontend+i18n, (3) docs — miễn migration DB không để user kẹt giữa hai bản schema lệch IPC.
- Tài liệu liên quan: [`interop.md`](./interop.md), [`DEVELOPMENT_PLAN.md`](./DEVELOPMENT_PLAN.md), `CHANGELOG.md` (khi ship).
