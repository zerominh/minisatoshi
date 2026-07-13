#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedAddress {
    pub address: String,
    pub index: u32,
    pub is_change: bool,
}
