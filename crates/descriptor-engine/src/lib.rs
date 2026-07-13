//! Compile Miniscript policies into Bitcoin output descriptors.

mod compile;
mod error;

pub use compile::{
    compile_descriptor_from_abstract, compile_descriptor_from_config, descriptor_checksum,
    parse_descriptor, NUMS_UNSPENDABLE_KEY,
};
pub use error::DescriptorError;
