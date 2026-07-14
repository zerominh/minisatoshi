use bitcoin::psbt::Psbt;

use crate::error::PsbtError;

pub fn combine_psbt(mut base: Psbt, other: Psbt) -> Result<Psbt, PsbtError> {
    base.combine(other)?;
    Ok(base)
}
