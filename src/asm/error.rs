#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("line {line}: {message}")]
pub struct AsmError {
    pub line: usize,
    pub message: String,
}

impl AsmError {
    pub fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_displays_with_line_number() {
        let e = AsmError::new(7, "unknown mnemonic 'LDZ'");
        assert_eq!(e.to_string(), "line 7: unknown mnemonic 'LDZ'");
    }
}
