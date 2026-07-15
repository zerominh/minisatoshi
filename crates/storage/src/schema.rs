pub const SCHEMA_VERSION: u32 = 2;

/// Fresh schema for brand-new databases that never had any prior schema.
pub const SCHEMA_V2: &str = r#"
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS workspaces (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    network     TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS wallets (
    id            TEXT PRIMARY KEY,
    workspace_id  TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name          TEXT NOT NULL,
    policy_json   TEXT NOT NULL,
    descriptor    TEXT NOT NULL,
    script_type   TEXT NOT NULL,
    created_at    INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS addresses (
    id          TEXT PRIMARY KEY,
    wallet_id   TEXT NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    address     TEXT NOT NULL,
    index_num   INTEGER NOT NULL,
    is_change   INTEGER NOT NULL,
    used        INTEGER NOT NULL DEFAULT 0,
    created_at  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS transactions (
    txid          TEXT NOT NULL,
    wallet_id     TEXT NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    block_height  INTEGER,
    amount        INTEGER,
    fee           INTEGER,
    confirmed     INTEGER,
    raw_json      TEXT,
    PRIMARY KEY (txid, wallet_id)
);

CREATE TABLE IF NOT EXISTS labels (
    id          TEXT PRIMARY KEY,
    target_type TEXT NOT NULL,
    target_id   TEXT NOT NULL,
    label       TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_wallets_workspace_id ON wallets(workspace_id);
CREATE INDEX IF NOT EXISTS idx_addresses_wallet_id ON addresses(wallet_id);
CREATE INDEX IF NOT EXISTS idx_transactions_wallet_id ON transactions(wallet_id);
"#;

/// Migrates a v1 database (old `wallets`/`vaults` container/spendable naming)
/// to the v2 `workspaces`/`wallets` naming:
/// - `wallets` (container)      -> `workspaces`
/// - `vaults` (spendable)       -> `wallets`, column `wallet_id` -> `workspace_id`
/// - `addresses.vault_id`       -> `addresses.wallet_id`
/// - `transactions.vault_id`    -> `transactions.wallet_id`
pub const MIGRATION_V2: &str = r#"
ALTER TABLE wallets RENAME TO workspaces;
ALTER TABLE vaults RENAME TO wallets;
ALTER TABLE wallets RENAME COLUMN wallet_id TO workspace_id;
ALTER TABLE addresses RENAME COLUMN vault_id TO wallet_id;
ALTER TABLE transactions RENAME COLUMN vault_id TO wallet_id;

DROP INDEX IF EXISTS idx_vaults_wallet_id;
DROP INDEX IF EXISTS idx_addresses_vault_id;
DROP INDEX IF EXISTS idx_transactions_vault_id;

CREATE INDEX IF NOT EXISTS idx_wallets_workspace_id ON wallets(workspace_id);
CREATE INDEX IF NOT EXISTS idx_addresses_wallet_id ON addresses(wallet_id);
CREATE INDEX IF NOT EXISTS idx_transactions_wallet_id ON transactions(wallet_id);
"#;

/// Old (pre-rename) v1 schema, kept only so tests can exercise the v1 -> v2
/// migration path against a database that looks like it did before Plan A.
#[cfg(test)]
pub mod legacy_v1 {
    pub const MIGRATION_V1: &str = r#"
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS wallets (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    network     TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS vaults (
    id            TEXT PRIMARY KEY,
    wallet_id     TEXT NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    name          TEXT NOT NULL,
    policy_json   TEXT NOT NULL,
    descriptor    TEXT NOT NULL,
    script_type   TEXT NOT NULL,
    created_at    INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS addresses (
    id          TEXT PRIMARY KEY,
    vault_id    TEXT NOT NULL REFERENCES vaults(id) ON DELETE CASCADE,
    address     TEXT NOT NULL,
    index_num   INTEGER NOT NULL,
    is_change   INTEGER NOT NULL,
    used        INTEGER NOT NULL DEFAULT 0,
    created_at  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS transactions (
    txid          TEXT NOT NULL,
    vault_id      TEXT NOT NULL REFERENCES vaults(id) ON DELETE CASCADE,
    block_height  INTEGER,
    amount        INTEGER,
    fee           INTEGER,
    confirmed     INTEGER,
    raw_json      TEXT,
    PRIMARY KEY (txid, vault_id)
);

CREATE TABLE IF NOT EXISTS labels (
    id          TEXT PRIMARY KEY,
    target_type TEXT NOT NULL,
    target_id   TEXT NOT NULL,
    label       TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_vaults_wallet_id ON vaults(wallet_id);
CREATE INDEX IF NOT EXISTS idx_addresses_vault_id ON addresses(vault_id);
CREATE INDEX IF NOT EXISTS idx_transactions_vault_id ON transactions(vault_id);
"#;
}
