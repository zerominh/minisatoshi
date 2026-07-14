use bitcoin::psbt::Psbt;
use bitcoin::secp256k1::Secp256k1;
use miniscript::descriptor::DescriptorSecretKey;
use miniscript::descriptor::KeyMap;

use crate::error::PsbtError;
use crate::types::SignProgress;

/// Software signer backed by a miniscript secret key or key map.
pub trait Signer: Send + Sync {
    fn sign_psbt(&self, psbt: &mut Psbt) -> Result<usize, PsbtError>;
}

pub struct SoftwareSigner {
    key: SignerKey,
}

enum SignerKey {
    Single(DescriptorSecretKey),
    Map(KeyMap),
}

impl SoftwareSigner {
    pub fn from_secret(key: DescriptorSecretKey) -> Self {
        Self {
            key: SignerKey::Single(key),
        }
    }

    pub fn from_key_map(map: KeyMap) -> Self {
        Self {
            key: SignerKey::Map(map),
        }
    }
}

impl Signer for SoftwareSigner {
    fn sign_psbt(&self, psbt: &mut Psbt) -> Result<usize, PsbtError> {
        let secp = Secp256k1::new();
        let signed = match &self.key {
            SignerKey::Single(key) => psbt.sign(key, &secp).map_err(|(_, errors)| {
                PsbtError::Signing(format!(
                    "could not sign one or more inputs ({} failure(s))",
                    errors.len()
                ))
            })?,
            SignerKey::Map(map) => psbt.sign(map, &secp).map_err(|(_, errors)| {
                PsbtError::Signing(format!(
                    "could not sign one or more inputs ({} failure(s))",
                    errors.len()
                ))
            })?,
        };
        verify_tap_script_sigs(psbt)?;
        Ok(signed.len())
    }
}

/// Reject PSBTs where taproot script-path signatures don't verify (wrong key / stale tx).
fn verify_tap_script_sigs(psbt: &Psbt) -> Result<(), PsbtError> {
    use bitcoin::hashes::Hash;
    use bitcoin::secp256k1::Message;
    use bitcoin::sighash::{Prevouts, SighashCache};

    let secp = Secp256k1::verification_only();
    let prevouts: Result<Vec<_>, _> = (0..psbt.inputs.len())
        .map(|i| {
            psbt.spend_utxo(i)
                .map(|o| o.clone())
                .map_err(|_| PsbtError::Signing("missing spend utxo while verifying signature".into()))
        })
        .collect();
    let prevouts = prevouts?;
    let mut cache = SighashCache::new(&psbt.unsigned_tx);

    for (index, input) in psbt.inputs.iter().enumerate() {
        for ((pk, leaf), sig) in &input.tap_script_sigs {
            let msg = cache
                .taproot_script_spend_signature_hash(
                    index,
                    &Prevouts::All(&prevouts),
                    *leaf,
                    sig.sighash_type,
                )
                .map_err(|e| PsbtError::Signing(format!("sighash failed while verifying: {e}")))?;
            secp.verify_schnorr(
                &sig.signature,
                &Message::from_digest(msg.to_byte_array()),
                pk,
            )
            .map_err(|_| {
                PsbtError::Signing(format!(
                    "invalid taproot script signature for key {pk} (input {index}) — wrong private key or PSBT changed after signing"
                ))
            })?;
        }
    }
    Ok(())
}

pub fn sign_psbt(psbt: &mut Psbt, signer: &dyn Signer) -> Result<SignProgress, PsbtError> {
    let total_inputs = psbt.inputs.len();
    let signed_inputs = signer.sign_psbt(psbt)?;
    Ok(SignProgress {
        signed_inputs,
        total_inputs,
    })
}
