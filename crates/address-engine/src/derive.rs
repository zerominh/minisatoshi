use bitcoin::address::Address;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::Network;
use descriptor_engine::compile_descriptor_from_abstract;
use miniscript::descriptor::Descriptor;
use miniscript::DescriptorPublicKey;
use policy_engine::{NetworkName, PolicyConfig};

use crate::error::AddressError;
use crate::types::DerivedAddress;

/// Derive a receive or change address at `index` from an output descriptor string.
///
/// Note: some compiled Taproot descriptors (e.g. ABC preset with padded taptrees)
/// cannot be round-tripped through the descriptor parser. Prefer
/// [`derive_address_from_policy`] when a [`PolicyConfig`] is available.
pub fn derive_address(
    descriptor: &str,
    network: NetworkName,
    index: u32,
    is_change: bool,
) -> Result<DerivedAddress, AddressError> {
    let desc = descriptor
        .parse::<Descriptor<DescriptorPublicKey>>()
        .map_err(|e| AddressError::Parse(e.to_string()))?;
    derive_address_from_descriptor(&desc, network, index, is_change)
}

/// Derive an address from a policy configuration (recompiles descriptor in memory).
pub fn derive_address_from_policy(
    policy: &PolicyConfig,
    index: u32,
    is_change: bool,
) -> Result<DerivedAddress, AddressError> {
    let desc = compile_descriptor_from_abstract(policy)
        .map_err(|e| AddressError::Parse(e.to_string()))?;
    derive_address_from_descriptor(&desc, policy.network, index, is_change)
}

/// Derive an address using policy when possible, otherwise fall back to the stored descriptor.
pub fn derive_address_for_vault(
    policy: &PolicyConfig,
    descriptor: &str,
    index: u32,
    is_change: bool,
) -> Result<DerivedAddress, AddressError> {
    if policy.keys.is_empty() {
        derive_address(descriptor, policy.network, index, is_change)
    } else {
        derive_address_from_policy(policy, index, is_change)
    }
}

/// Derive a receive address for a vault descriptor.
pub fn new_receive_address(
    policy: &PolicyConfig,
    descriptor: &str,
    index: u32,
) -> Result<DerivedAddress, AddressError> {
    derive_address_for_vault(policy, descriptor, index, false)
}

/// Derive a change address for a vault descriptor.
pub fn new_change_address(
    policy: &PolicyConfig,
    descriptor: &str,
    index: u32,
) -> Result<DerivedAddress, AddressError> {
    derive_address_for_vault(policy, descriptor, index, true)
}

pub fn derive_address_from_descriptor(
    descriptor: &Descriptor<DescriptorPublicKey>,
    network: NetworkName,
    index: u32,
    is_change: bool,
) -> Result<DerivedAddress, AddressError> {
    let bitcoin_network = network.to_bitcoin_network();
    let address = derive_address_string(descriptor, bitcoin_network, index, is_change)?;

    Ok(DerivedAddress {
        address,
        index,
        is_change,
    })
}

fn derive_address_string(
    descriptor: &Descriptor<DescriptorPublicKey>,
    network: Network,
    index: u32,
    is_change: bool,
) -> Result<String, AddressError> {
    let secp = Secp256k1::verification_only();
    let chain_descriptor = select_chain_descriptor(descriptor, is_change)?;

    let derived = chain_descriptor
        .at_derivation_index(index)
        .map_err(|e| AddressError::Derivation(e.to_string()))?
        .derived_descriptor(&secp);

    let script = derived.script_pubkey();
    let address = Address::from_script(&script, network)
        .map_err(|e| AddressError::Encoding(e.to_string()))?;

    Ok(address.to_string())
}

fn select_chain_descriptor(
    descriptor: &Descriptor<DescriptorPublicKey>,
    is_change: bool,
) -> Result<Descriptor<DescriptorPublicKey>, AddressError> {
    if descriptor.is_multipath() {
        let singles = descriptor
            .clone()
            .into_single_descriptors()
            .map_err(|e| AddressError::Derivation(e.to_string()))?;

        let chain_index = if is_change { 1 } else { 0 };
        singles.into_iter().nth(chain_index).ok_or(AddressError::UnsupportedMultipath {
            is_change,
        })
    } else if is_change {
        Err(AddressError::UnsupportedMultipath { is_change: true })
    } else {
        Ok(descriptor.clone())
    }
}

#[cfg(test)]
mod tests {
    use descriptor_engine::compile_descriptor_from_config;
    use policy_engine::{
        abc_preset, test_vectors::TEST_FP, test_vectors::TEST_XPUB_A, test_vectors::TEST_XPUB_B,
        test_vectors::TEST_XPUB_C, KeyConfig, KeyRole, NetworkName,
    };
    use wallet_core::WalletStore;

    use super::*;

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

    #[test]
    fn derives_receive_and_change_addresses() {
        let keys = sample_keys();
        let policy = abc_preset(
            keys[0].clone(),
            keys[1].clone(),
            keys[2].clone(),
            4,
            NetworkName::Testnet,
        );
        let descriptor = compile_descriptor_from_config(&policy).unwrap();

        let receive = new_receive_address(&policy, &descriptor, 0).unwrap();
        let change = new_change_address(&policy, &descriptor, 0).unwrap();

        assert!(receive.address.starts_with("tb1"));
        assert!(change.address.starts_with("tb1"));
        assert_ne!(receive.address, change.address);
    }

    #[test]
    fn vault_policy_to_first_receive_address() {
        let dir = tempfile::tempdir().unwrap();
        let store = WalletStore::open(dir.path().join("wallet.db")).unwrap();
        let wallet = store.create_wallet("Vault", NetworkName::Testnet).unwrap();

        let keys = sample_keys();
        let policy = abc_preset(
            keys[0].clone(),
            keys[1].clone(),
            keys[2].clone(),
            4,
            NetworkName::Testnet,
        );
        let vault = store.create_vault(&wallet.id, "ABC", policy).unwrap();

        let address = new_receive_address(&vault.policy, &vault.descriptor, 0).unwrap();
        assert!(address.address.starts_with("tb1"));
    }
}
