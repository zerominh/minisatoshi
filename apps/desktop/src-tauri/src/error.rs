//! Map internal errors to safe, user-facing strings for Tauri IPC.

/// Convert any displayable error into a UI-safe message.
///
/// Strips extended private keys (`xprv` / `tprv`) so they never reach the frontend.
pub fn user_facing_error(err: impl std::fmt::Display) -> String {
    redact_secrets(&err.to_string())
}

/// Replace BIP32 private-key material with a placeholder.
pub fn redact_secrets(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    let bytes = message.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if looks_like_xprv_or_tprv(bytes, i) {
            out.push_str("[redacted-private-key]");
            i = skip_base58(bytes, i);
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn looks_like_xprv_or_tprv(bytes: &[u8], i: usize) -> bool {
    let rest = &bytes[i..];
    rest.len() >= 4
        && (rest.starts_with(b"xprv")
            || rest.starts_with(b"tprv")
            || rest.starts_with(b"XPRV")
            || rest.starts_with(b"TPRV"))
}

fn skip_base58(bytes: &[u8], start: usize) -> usize {
    let mut i = start;
    while i < bytes.len() && is_base58(bytes[i]) {
        i += 1;
    }
    i
}

fn is_base58(b: u8) -> bool {
    matches!(b,
        b'1'..=b'9' | b'A'..=b'H' | b'J'..=b'N' | b'P'..=b'Z' | b'a'..=b'k' | b'm'..=b'z'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_xprv_material() {
        let leaked =
            "signing failed with xprv9s21ZrQH143K3QTDL4L64na6s5k8gvS7vKqG8JQVZGvqVZGvqVZGvqVZGvqVZ";
        let safe = redact_secrets(leaked);
        assert!(!safe.contains("xprv9"));
        assert!(safe.contains("[redacted-private-key]"));
    }

    #[test]
    fn redacts_tprv_material() {
        let leaked = format!("bad key tprv{}", "A".repeat(100));
        let safe = redact_secrets(&leaked);
        assert!(!safe.to_lowercase().contains("tprv"));
        assert!(safe.contains("[redacted-private-key]"));
    }

    #[test]
    fn keeps_normal_errors() {
        assert_eq!(
            redact_secrets("insufficient funds: need 100 sats, have 50 sats"),
            "insufficient funds: need 100 sats, have 50 sats"
        );
    }

    #[test]
    fn user_facing_error_redacts_wrapped_display() {
        #[derive(Debug)]
        struct Wrapped(&'static str);
        impl std::fmt::Display for Wrapped {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "signing error: {}", self.0)
            }
        }

        let msg = user_facing_error(Wrapped(
            "tprvABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz123456789",
        ));
        assert!(msg.contains("[redacted-private-key]"));
        assert!(!msg.contains("tprv"));
    }
}
