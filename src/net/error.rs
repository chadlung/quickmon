#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NetError {
    #[error("Cannot reach the device: {0}")]
    Transport(String),

    #[error("Device returned HTTP {status}")]
    Http { status: u16 },

    #[error("Device reported: {}", errors.join("; "))]
    Api { errors: Vec<String> },

    #[error("{len} bytes at ${address:04X} would run past $FFFF")]
    WouldWrap { address: u16, len: usize },
}
