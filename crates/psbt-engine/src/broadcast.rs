use bitcoin::psbt::Psbt;
use blockchain::BlockchainBackend;

use crate::error::PsbtError;
use crate::finalize::{finalize_psbt, transaction_hex};

pub fn broadcast_psbt(psbt: &Psbt, backend: &dyn BlockchainBackend) -> Result<String, PsbtError> {
    let mut working = psbt.clone();
    let tx = finalize_psbt(&mut working)?;
    let hex = transaction_hex(&tx);
    backend.broadcast(&hex).map_err(PsbtError::from)
}
