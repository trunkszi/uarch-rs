#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DetectError {
    #[error("the current execution domain could not be verified")]
    UnverifiedExecutionDomain,
    #[error("the requested instruction is unsupported")]
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AuditError {
    #[error("host evidence is unavailable")]
    Unavailable,
    #[error("host evidence is malformed")]
    Malformed,
}
