use bitcoin::consensus::encode::serialize;
use bitcoin::psbt::Psbt;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::Transaction;
use miniscript::psbt::PsbtExt;

use crate::error::PsbtError;

pub fn finalize_psbt(psbt: &mut Psbt) -> Result<Transaction, PsbtError> {
    let secp = Secp256k1::verification_only();
    psbt.finalize_mut(&secp).map_err(|errors| {
        let details: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        PsbtError::Finalize(format!(
            "could not finalize PSBT ({} failure(s)): {}",
            errors.len(),
            details.join("; ")
        ))
    })?;

    psbt.extract(&secp)
        .map_err(|_| PsbtError::Finalize("could not extract finalized transaction".into()))
}

pub fn extract_transaction(psbt: &Psbt) -> Result<Transaction, PsbtError> {
    if psbt.inputs.iter().any(|input| {
        input.final_script_witness.is_none()
            && input
                .final_script_sig
                .as_ref()
                .is_none_or(|script| script.is_empty())
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
