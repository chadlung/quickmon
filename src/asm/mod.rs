pub mod encode;
pub mod format;
pub mod error;
pub mod opcodes;
pub mod parser;

pub use encode::{Assembly, ListingLine};
pub use error::AsmError;
pub use opcodes::{opcode, AddrMode, Mnemonic};

/// Assemble 6510 source at `org`, returning the emitted bytes and a listing.
pub fn assemble(src: &str, org: u16) -> Result<Assembly, Vec<AsmError>> {
    let lines = parser::parse(src)?;
    encode::encode(&lines, org)
}
