//! Map ledger-bitcoin / device errors to user-facing `SignError` variants.

use crate::error::SignError;

/// Normalize raw ledger-bitcoin / Ledger device errors for the UI.
pub fn map_ledger_cli_error(raw: &str) -> SignError {
    let trimmed = raw.trim();
    let lower = trimmed.to_ascii_lowercase();

    if is_user_cancelled(&lower) {
        return SignError::Cancelled;
    }
    if lower.contains("timeout") || lower.contains("timed out") {
        return SignError::Ledger(
            "Ledger did not respond in time. Unlock the device, open Bitcoin / Bitcoin Test, \
             close Ledger Live (it locks USB), then retry."
                .into(),
        );
    }
    if lower.contains("0x5515") || lower.contains("5515") {
        return SignError::Ledger(
            "Ledger is locked (0x5515) — enter your PIN, then open Bitcoin Test (or Bitcoin)."
                .into(),
        );
    }
    if lower.contains("wrong network")
        || lower.contains("network mismatch")
        || (lower.contains("chain") && lower.contains("mismatch"))
    {
        return SignError::Ledger(
            "Ledger network mismatch — ensure the wallet network matches the Bitcoin app (mainnet vs testnet)."
                .into(),
        );
    }
    if lower.contains("firmware")
        || lower.contains("outdated")
        || lower.contains("version not supported")
        || lower.contains("app version")
    {
        return SignError::Ledger(
            "Ledger Bitcoin app or firmware may be too old for Miniscript wallet policies. \
             Update the Bitcoin app (≥ 2.1) and device firmware, then retry."
                .into(),
        );
    }
    if lower.contains("hmac")
        || lower.contains("not registered")
        || lower.contains("unknown wallet")
        || lower.contains("wallet policy")
            && (lower.contains("not found") || lower.contains("missing"))
    {
        return SignError::Ledger(
            "Ledger wallet policy not registered for this vault — Wallet → Settings → Register Ledger policy."
                .into(),
        );
    }
    if lower.contains("0x6a82") || lower.contains("6a82") {
        return SignError::Ledger(
            "Ledger rejected a key derivation path (0x6a82). Bitcoin app ≥ 2.4.3 only allows \
             standard BIP paths with the correct coin type (0' mainnet, 1' testnet). \
             Use origin paths like 86'/1'/0' on Bitcoin Test, or install Bitcoin Recovery \
             temporarily for non-standard paths."
                .into(),
        );
    }
    if lower.contains("0x6a80") || lower.contains("6a80") {
        return SignError::Ledger(
            "Ledger rejected the wallet policy (0x6a80). Open the correct Bitcoin app for your \
             network (Bitcoin vs Bitcoin Test), ensure app version ≥ 2.2.1, unlock the device, \
             and retry Register Ledger policy."
                .into(),
        );
    }
    if lower.contains("hidapinotinstalled")
        || lower.contains("hidapi is not installed")
        || lower.contains("ledgercomm[hid]")
    {
        return SignError::Ledger(
            "Ledger USB support missing in the bundled signer — Settings → Install Ledger signer again."
                .into(),
        );
    }
    if lower.contains("disconnected")
        || lower.contains("device not found")
        || lower.contains("no device")
        || lower.contains("failed to open")
    {
        return SignError::DeviceNotFound(
            "Ledger not found — reconnect USB, unlock the device, and open the Bitcoin app.".into(),
        );
    }
    if lower.contains("rejected") || lower.contains("denied") {
        return SignError::Ledger(
            "Ledger rejected the request — check the policy on screen and approve, or re-register if the descriptor changed."
                .into(),
        );
    }

    SignError::Ledger(trimmed.to_string())
}

fn is_user_cancelled(lower: &str) -> bool {
    lower.contains("cancel")
        || lower.contains("user refused")
        || lower.contains("user denied")
        || lower.contains("conditions of use not satisfied")
        || lower.contains("0x6985")
        || lower.contains("0x5501")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_cancel_to_sign_error() {
        assert_eq!(
            map_ledger_cli_error("User cancelled"),
            SignError::Cancelled
        );
    }

    #[test]
    fn maps_timeout() {
        let err = map_ledger_cli_error("operation timed out");
        assert!(matches!(err, SignError::Ledger(_)));
        assert!(err.to_string().contains("did not respond"));
    }

    #[test]
    fn maps_hmac_missing() {
        let err = map_ledger_cli_error("missing hmac in request");
        assert!(err.to_string().contains("Register Ledger policy"));
    }

    #[test]
    fn maps_hidapi_missing() {
        let err = map_ledger_cli_error("HIDAPINotInstalledError: hidapi is not installed");
        assert!(err.to_string().contains("Install Ledger signer"));
    }

    #[test]
    fn maps_6a82_path_hardening() {
        let err = map_ledger_cli_error(
            "('0x6a82', 'Error in <BitcoinInsType.REGISTER_WALLET: 2> command', '')",
        );
        assert!(err.to_string().contains("0x6a82"));
        assert!(err.to_string().contains("coin type"));
    }
}
