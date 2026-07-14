use std::str::FromStr;

use miniscript::DescriptorPublicKey;

use crate::config::{KeyConfig, KeyRole};
use crate::test_vectors::{TEST_FP, TEST_XPUB_A, TEST_XPUB_B, TEST_XPUB_C};
use crate::translate::descriptor_key_expression;

#[test]
fn descriptor_key_expression_parses_for_test_vectors() {
    for key in sample_keys() {
        let expr = descriptor_key_expression(&key).unwrap();
        let parsed = DescriptorPublicKey::from_str(&expr);
        assert!(
            parsed.is_ok(),
            "failed for {}: {expr} => {parsed:?}",
            key.id
        );
    }
}

fn sample_keys() -> [KeyConfig; 3] {
    [
        KeyConfig {
            id: "A".into(),
            role: KeyRole::Investor,
            xpub: TEST_XPUB_A.into(),
            fingerprint: "78412e3a".into(),
            origin_path: Some("44'/0'/0'".into()),
        },
        KeyConfig {
            id: "B".into(),
            role: KeyRole::Manager,
            xpub: TEST_XPUB_B.into(),
            fingerprint: TEST_FP.into(),
            origin_path: Some("86'/0'/0'".into()),
        },
        KeyConfig {
            id: "C".into(),
            role: KeyRole::Recovery,
            xpub: TEST_XPUB_C.into(),
            fingerprint: TEST_FP.into(),
            origin_path: Some("84'/0'/0'".into()),
        },
    ]
}
