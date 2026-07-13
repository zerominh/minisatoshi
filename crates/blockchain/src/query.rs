use policy_engine::PolicyConfig;

/// Inputs required to scan or query a vault on-chain.
#[derive(Debug, Clone)]
pub struct DescriptorQuery {
    pub policy: PolicyConfig,
    pub descriptor: String,
}

impl DescriptorQuery {
    pub fn new(policy: PolicyConfig, descriptor: impl Into<String>) -> Self {
        Self {
            policy,
            descriptor: descriptor.into(),
        }
    }

    pub fn network(&self) -> policy_engine::NetworkName {
        self.policy.network
    }
}
