//! Helpers for validating hardware (HWI) signing progress.

use std::collections::BTreeSet;

use bitcoin::psbt::Psbt;

use crate::status::signed_fingerprints;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureSnapshot {
    pub fingerprints: BTreeSet<String>,
    pub tap_script_sig_count: usize,
    pub tap_key_sig_count: usize,
}

pub fn signature_snapshot(psbt: &Psbt) -> SignatureSnapshot {
    let mut tap_script_sig_count = 0usize;
    let mut tap_key_sig_count = 0usize;
    for input in &psbt.inputs {
        tap_script_sig_count += input.tap_script_sigs.len();
        if input.tap_key_sig.is_some() {
            tap_key_sig_count += 1;
        }
    }
    SignatureSnapshot {
        fingerprints: signed_fingerprints(psbt),
        tap_script_sig_count,
        tap_key_sig_count,
    }
}

/// True when HWI returned a PSBT with at least one new signature or fingerprint.
pub fn hw_sign_made_progress(before: &SignatureSnapshot, after: &SignatureSnapshot) -> bool {
    if after.tap_script_sig_count > before.tap_script_sig_count {
        return true;
    }
    if after.tap_key_sig_count > before.tap_key_sig_count {
        return true;
    }
    if !after.fingerprints.is_empty() && after.fingerprints != before.fingerprints {
        return true;
    }
    false
}
