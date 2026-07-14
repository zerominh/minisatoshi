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
            SignerKey::Single(key) => psbt
                .sign(key, &secp)
                .map_err(|(_, errors)| PsbtError::Signing(format!("{errors:?}")))?,
            SignerKey::Map(map) => psbt
                .sign(map, &secp)
                .map_err(|(_, errors)| PsbtError::Signing(format!("{errors:?}")))?,
        };
        Ok(signed.len())
    }
}

pub fn sign_psbt(psbt: &mut Psbt, signer: &dyn Signer) -> Result<SignProgress, PsbtError> {
    let total_inputs = psbt.inputs.len();
    let signed_inputs = signer.sign_psbt(psbt)?;
    Ok(SignProgress {
        signed_inputs,
        total_inputs,
    })
}
