use bitcoin::consensus::encode::serialize;
use bitcoin::psbt::Psbt;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::Transaction;
use miniscript::psbt::PsbtExt;

use crate::error::PsbtError;

pub fn finalize_psbt(psbt: &mut Psbt) -> Result<Transaction, PsbtError> {
    let secp = Secp256k1::verification_only();
    psbt.finalize_mut(&secp)
        .map_err(|errors| PsbtError::Finalize(format!("{errors:?}")))?;

    psbt.extract(&secp).map_err(|e| PsbtError::Finalize(e.to_string()))
}

pub fn extract_transaction(psbt: &Psbt) -> Result<Transaction, PsbtError> {
    if psbt.inputs.iter().any(|input| {
        input.final_script_witness.is_none()
            && input
                .final_script_sig
                .as_ref()
                .map_or(true, |script| script.is_empty())
    }) {
        return Err(PsbtError::NotFinalized);
    }

    Ok(psbt.unsigned_tx.clone())
}

pub fn transaction_hex(tx: &Transaction) -> String {
    bitcoin::consensus::encode::serialize_hex(tx)
}

pub fn transaction_bytes(tx: &Transaction) -> Vec<u8> {
    serialize(tx)
}
