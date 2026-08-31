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

    /// A readmem body whose length is not what was asked for. The device
    /// answered, so this is neither a transport nor an HTTP failure — it is
    /// the device not honouring the documented contract, and treating a short
    /// or overlong body as data would quietly corrupt every comparison and
    /// hex dump built from it.
    #[error("Device returned {actual} bytes, expected {expected}")]
    UnexpectedLength { expected: usize, actual: usize },
}
