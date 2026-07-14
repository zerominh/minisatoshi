use blockchain::Utxo;
use wallet_core::Vault;

/// UTXO selected for spending, with derivation metadata for descriptor lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpendingUtxo {
    pub utxo: Utxo,
    pub derivation_index: u32,
    pub is_change: bool,
}

impl SpendingUtxo {
    pub fn new(utxo: Utxo, derivation_index: u32, is_change: bool) -> Self {
        Self {
            utxo,
            derivation_index,
            is_change,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PsbtRecipient {
    pub address: String,
    pub amount_sats: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeeRate {
    pub sat_per_vb: u64,
}

impl FeeRate {
    pub fn new(sat_per_vb: u64) -> Self {
        Self { sat_per_vb }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CreatePsbtOptions {
    /// Relative locktime (BIP68) applied to all inputs, e.g. for `older(N)` paths.
    pub input_sequence: Option<u32>,
    /// Change address derivation index (default 0).
    pub change_index: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignProgress {
    pub signed_inputs: usize,
    pub total_inputs: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// PSBT as a base64 string (Sparrow paste/import).
    Base64,
    /// Raw PSBT binary (.psbt file bytes).
    File,
}

pub type Psbt = bitcoin::Psbt;

pub struct VaultPsbt<'a> {
    pub vault: &'a Vault,
}

impl<'a> VaultPsbt<'a> {
    pub fn new(vault: &'a Vault) -> Self {
        Self { vault }
    }
}
