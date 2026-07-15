use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceRecord {
    pub id: String,
    pub name: String,
    pub network: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewWorkspace {
    pub id: String,
    pub name: String,
    pub network: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalletRecord {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub policy_json: String,
    pub descriptor: String,
    pub script_type: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewWallet {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub policy_json: String,
    pub descriptor: String,
    pub script_type: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddressRecord {
    pub id: String,
    pub wallet_id: String,
    pub address: String,
    pub index: u32,
    pub is_change: bool,
    pub used: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAddress {
    pub id: String,
    pub wallet_id: String,
    pub address: String,
    pub index: u32,
    pub is_change: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionRecord {
    pub txid: String,
    pub wallet_id: String,
    pub block_height: Option<i64>,
    pub amount: Option<i64>,
    pub fee: Option<i64>,
    pub confirmed: Option<bool>,
    pub raw_json: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTransaction {
    pub txid: String,
    pub wallet_id: String,
    pub block_height: Option<i64>,
    pub amount: Option<i64>,
    pub fee: Option<i64>,
    pub confirmed: Option<bool>,
    pub raw_json: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabelRecord {
    pub id: String,
    pub target_type: String,
    pub target_id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewLabel {
    pub id: String,
    pub target_type: String,
    pub target_id: String,
    pub label: String,
}
