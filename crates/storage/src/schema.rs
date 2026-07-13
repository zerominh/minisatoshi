pub const SCHEMA_VERSION: u32 = 1;

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
