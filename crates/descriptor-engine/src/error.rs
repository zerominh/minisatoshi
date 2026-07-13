use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DescriptorError {
    #[error("policy error: {0}")]
    Policy(#[from] policy_engine::PolicyError),

    #[error("descriptor parse error: {0}")]
    Parse(String),

    #[error("descriptor compile error: {0}")]
    Compile(String),

    #[error("unsupported script type: {0}")]
    UnsupportedScriptType(String),
}
