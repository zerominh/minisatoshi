use descriptor_engine::compile_descriptor_from_abstract;
use miniscript::descriptor::Descriptor;
use miniscript::DefiniteDescriptorKey;
use policy_engine::PolicyConfig;

use crate::error::PsbtError;

pub(crate) fn definite_descriptor_at(
    policy: &PolicyConfig,
    index: u32,
    is_change: bool,
) -> Result<Descriptor<DefiniteDescriptorKey>, PsbtError> {
    let desc = compile_descriptor_from_abstract(policy)?;
    let chain = select_chain_descriptor(&desc, is_change)?;
    chain.at_derivation_index(index).map_err(|e| {
        PsbtError::Descriptor(descriptor_engine::DescriptorError::Compile(e.to_string()))
    })
}

fn select_chain_descriptor(
    descriptor: &Descriptor<miniscript::DescriptorPublicKey>,
    is_change: bool,
) -> Result<Descriptor<miniscript::DescriptorPublicKey>, PsbtError> {
    if descriptor.is_multipath() {
        let singles = descriptor.clone().into_single_descriptors().map_err(|e| {
            PsbtError::Descriptor(descriptor_engine::DescriptorError::Compile(e.to_string()))
        })?;

        let chain_index = if is_change { 1 } else { 0 };
        singles.into_iter().nth(chain_index).ok_or_else(|| {
            PsbtError::Address(address_engine::AddressError::UnsupportedMultipath { is_change })
        })
    } else if is_change {
        Err(PsbtError::Address(
            address_engine::AddressError::UnsupportedMultipath { is_change: true },
        ))
    } else {
        Ok(descriptor.clone())
    }
}
