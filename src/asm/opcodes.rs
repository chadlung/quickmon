#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mnemonic {
    Adc,
    And,
    Asl,
    Bcc,
    Bcs,
    Beq,
    Bit,
    Bmi,
    Bne,
    Bpl,
    Brk,
    Bvc,
    Bvs,
    Clc,
    Cld,
    Cli,
    Clv,
    Cmp,
    Cpx,
    Cpy,
    Dec,
    Dex,
    Dey,
    Eor,
    Inc,
    Inx,
    Iny,
    Jmp,
    Jsr,
    Lda,
    Ldx,
    Ldy,
    Lsr,
    Nop,
    Ora,
    Pha,
    Php,
    Pla,
    Plp,
    Rol,
    Ror,
    Rti,
    Rts,
    Sbc,
    Sec,
    Sed,
    Sei,
    Sta,
    Stx,
    Sty,
    Tax,
    Tay,
    Tsx,
    Txa,
    Txs,
    Tya,
}

impl Mnemonic {
    pub fn parse(s: &str) -> Option<Self> {
        use Mnemonic::*;
        Some(match s.to_ascii_uppercase().as_str() {
            "ADC" => Adc,
            "AND" => And,
            "ASL" => Asl,
            "BCC" => Bcc,
            "BCS" => Bcs,
            "BEQ" => Beq,
            "BIT" => Bit,
            "BMI" => Bmi,
            "BNE" => Bne,
            "BPL" => Bpl,
            "BRK" => Brk,
            "BVC" => Bvc,
            "BVS" => Bvs,
            "CLC" => Clc,
            "CLD" => Cld,
            "CLI" => Cli,
            "CLV" => Clv,
            "CMP" => Cmp,
            "CPX" => Cpx,
            "CPY" => Cpy,
            "DEC" => Dec,
            "DEX" => Dex,
            "DEY" => Dey,
            "EOR" => Eor,
            "INC" => Inc,
            "INX" => Inx,
            "INY" => Iny,
            "JMP" => Jmp,
            "JSR" => Jsr,
            "LDA" => Lda,
            "LDX" => Ldx,
            "LDY" => Ldy,
            "LSR" => Lsr,
            "NOP" => Nop,
            "ORA" => Ora,
            "PHA" => Pha,
            "PHP" => Php,
            "PLA" => Pla,
            "PLP" => Plp,
            "ROL" => Rol,
            "ROR" => Ror,
            "RTI" => Rti,
            "RTS" => Rts,
            "SBC" => Sbc,
            "SEC" => Sec,
            "SED" => Sed,
            "SEI" => Sei,
            "STA" => Sta,
            "STX" => Stx,
            "STY" => Sty,
            "TAX" => Tax,
            "TAY" => Tay,
            "TSX" => Tsx,
            "TXA" => Txa,
            "TXS" => Txs,
            "TYA" => Tya,
            _ => return None,
        })
    }

    pub fn is_branch(self) -> bool {
        use Mnemonic::*;
        matches!(self, Bcc | Bcs | Beq | Bmi | Bne | Bpl | Bvc | Bvs)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AddrMode {
    Implied,
    Accumulator,
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    IndexedIndirect,
    IndirectIndexed,
    Indirect,
    Relative,
}

impl AddrMode {
    pub fn operand_len(self) -> usize {
        use AddrMode::*;
        match self {
            Implied | Accumulator => 0,
            Absolute | AbsoluteX | AbsoluteY | Indirect => 2,
            Immediate | ZeroPage | ZeroPageX | ZeroPageY | IndexedIndirect | IndirectIndexed
            | Relative => 1,
        }
    }

    pub fn instr_len(self) -> usize {
        1 + self.operand_len()
    }
}

pub fn opcode(m: Mnemonic, mode: AddrMode) -> Option<u8> {
    OPCODES
        .iter()
        .find(|(mm, md, _)| *mm == m && *md == mode)
        .map(|(_, _, op)| *op)
}

// Transcribed from instruction-set-6510.md, mnemonic by mnemonic in the
// file's own order. A handful of entries below deviate from the file's
// literal per-mnemonic table text because that text contains OCR/transcription
// errors. The corrections are cross-checked against the c64-wiki opcode
// matrix (https://www.c64-wiki.com/wiki/Opcode) — an independent source, not
// derived from this same reference file — and, for STA Absolute specifically,
// against the hardware-verified bytes `8D 00 04` in tests/golden.rs, which
// were run on a real C64 before this project existed. See task-2-report.md,
// "Transcription notes", for the full list.
pub static OPCODES: &[(Mnemonic, AddrMode, u8)] = &[
    // ADC
    (Mnemonic::Adc, AddrMode::Immediate, 0x69),
    (Mnemonic::Adc, AddrMode::ZeroPage, 0x65),
    (Mnemonic::Adc, AddrMode::ZeroPageX, 0x75),
    (Mnemonic::Adc, AddrMode::Absolute, 0x6D), // file says 0x60 (typo); c64-wiki opcode matrix confirms 0x6D (0x60 is RTS)
    (Mnemonic::Adc, AddrMode::AbsoluteX, 0x7D), // file says 0x70 (typo); c64-wiki opcode matrix confirms 0x7D (0x70 is BVS)
    (Mnemonic::Adc, AddrMode::AbsoluteY, 0x79),
    (Mnemonic::Adc, AddrMode::IndexedIndirect, 0x61),
    (Mnemonic::Adc, AddrMode::IndirectIndexed, 0x71),
    // AND
    (Mnemonic::And, AddrMode::Immediate, 0x29),
    (Mnemonic::And, AddrMode::ZeroPage, 0x25),
    (Mnemonic::And, AddrMode::ZeroPageX, 0x35),
    (Mnemonic::And, AddrMode::Absolute, 0x2D),
    (Mnemonic::And, AddrMode::AbsoluteX, 0x3D),
    (Mnemonic::And, AddrMode::AbsoluteY, 0x39),
    (Mnemonic::And, AddrMode::IndexedIndirect, 0x21),
    (Mnemonic::And, AddrMode::IndirectIndexed, 0x31),
    // ASL
    (Mnemonic::Asl, AddrMode::Accumulator, 0x0A),
    (Mnemonic::Asl, AddrMode::ZeroPage, 0x06),
    (Mnemonic::Asl, AddrMode::ZeroPageX, 0x16),
    (Mnemonic::Asl, AddrMode::Absolute, 0x0E),
    (Mnemonic::Asl, AddrMode::AbsoluteX, 0x1E),
    // BCC
    (Mnemonic::Bcc, AddrMode::Relative, 0x90),
    // BCS
    (Mnemonic::Bcs, AddrMode::Relative, 0xB0),
    // BEQ
    (Mnemonic::Beq, AddrMode::Relative, 0xF0),
    // BIT
    (Mnemonic::Bit, AddrMode::ZeroPage, 0x24),
    (Mnemonic::Bit, AddrMode::Absolute, 0x2C),
    // BMI
    (Mnemonic::Bmi, AddrMode::Relative, 0x30),
    // BNE
    (Mnemonic::Bne, AddrMode::Relative, 0xD0),
    // BPL
    (Mnemonic::Bpl, AddrMode::Relative, 0x10),
    // BRK
    (Mnemonic::Brk, AddrMode::Implied, 0x00),
    // BVC
    (Mnemonic::Bvc, AddrMode::Relative, 0x50),
    // BVS
    (Mnemonic::Bvs, AddrMode::Relative, 0x70),
    // CLC
    (Mnemonic::Clc, AddrMode::Implied, 0x18),
    // CLD
    (Mnemonic::Cld, AddrMode::Implied, 0xD8),
    // CLI
    (Mnemonic::Cli, AddrMode::Implied, 0x58),
    // CLV
    (Mnemonic::Clv, AddrMode::Implied, 0xB8),
    // CMP
    (Mnemonic::Cmp, AddrMode::Immediate, 0xC9),
    (Mnemonic::Cmp, AddrMode::ZeroPage, 0xC5),
    (Mnemonic::Cmp, AddrMode::ZeroPageX, 0xD5),
    (Mnemonic::Cmp, AddrMode::Absolute, 0xCD),
    (Mnemonic::Cmp, AddrMode::AbsoluteX, 0xDD),
    (Mnemonic::Cmp, AddrMode::AbsoluteY, 0xD9),
    (Mnemonic::Cmp, AddrMode::IndexedIndirect, 0xC1),
    (Mnemonic::Cmp, AddrMode::IndirectIndexed, 0xD1),
    // CPX
    (Mnemonic::Cpx, AddrMode::Immediate, 0xE0),
    (Mnemonic::Cpx, AddrMode::ZeroPage, 0xE4),
    (Mnemonic::Cpx, AddrMode::Absolute, 0xEC),
    // CPY
    (Mnemonic::Cpy, AddrMode::Immediate, 0xC0),
    (Mnemonic::Cpy, AddrMode::ZeroPage, 0xC4),
    (Mnemonic::Cpy, AddrMode::Absolute, 0xCC),
    // DEC
    (Mnemonic::Dec, AddrMode::ZeroPage, 0xC6),
    (Mnemonic::Dec, AddrMode::ZeroPageX, 0xD6),
    (Mnemonic::Dec, AddrMode::Absolute, 0xCE),
    (Mnemonic::Dec, AddrMode::AbsoluteX, 0xDE),
    // DEX
    (Mnemonic::Dex, AddrMode::Implied, 0xCA),
    // DEY
    (Mnemonic::Dey, AddrMode::Implied, 0x88),
    // EOR
    (Mnemonic::Eor, AddrMode::Immediate, 0x49),
    (Mnemonic::Eor, AddrMode::ZeroPage, 0x45),
    (Mnemonic::Eor, AddrMode::ZeroPageX, 0x55),
    (Mnemonic::Eor, AddrMode::Absolute, 0x4D), // file says 0x40 (typo); c64-wiki opcode matrix confirms 0x4D (0x40 is RTI)
    (Mnemonic::Eor, AddrMode::AbsoluteX, 0x5D), // file says 0x50 (typo); c64-wiki opcode matrix confirms 0x5D (0x50 is BVC)
    (Mnemonic::Eor, AddrMode::AbsoluteY, 0x59),
    (Mnemonic::Eor, AddrMode::IndexedIndirect, 0x41),
    (Mnemonic::Eor, AddrMode::IndirectIndexed, 0x51),
    // INC
    (Mnemonic::Inc, AddrMode::ZeroPage, 0xE6),
    (Mnemonic::Inc, AddrMode::ZeroPageX, 0xF6),
    (Mnemonic::Inc, AddrMode::Absolute, 0xEE),
    (Mnemonic::Inc, AddrMode::AbsoluteX, 0xFE),
    // INX
    (Mnemonic::Inx, AddrMode::Implied, 0xE8),
    // INY
    (Mnemonic::Iny, AddrMode::Implied, 0xC8),
    // JMP
    (Mnemonic::Jmp, AddrMode::Absolute, 0x4C),
    (Mnemonic::Jmp, AddrMode::Indirect, 0x6C),
    // JSR
    (Mnemonic::Jsr, AddrMode::Absolute, 0x20),
    // LDA
    (Mnemonic::Lda, AddrMode::Immediate, 0xA9),
    (Mnemonic::Lda, AddrMode::ZeroPage, 0xA5),
    (Mnemonic::Lda, AddrMode::ZeroPageX, 0xB5),
    (Mnemonic::Lda, AddrMode::Absolute, 0xAD),
    (Mnemonic::Lda, AddrMode::AbsoluteX, 0xBD),
    (Mnemonic::Lda, AddrMode::AbsoluteY, 0xB9),
    (Mnemonic::Lda, AddrMode::IndexedIndirect, 0xA1),
    (Mnemonic::Lda, AddrMode::IndirectIndexed, 0xB1),
    // LDX
    (Mnemonic::Ldx, AddrMode::Immediate, 0xA2),
    (Mnemonic::Ldx, AddrMode::ZeroPage, 0xA6),
    (Mnemonic::Ldx, AddrMode::ZeroPageY, 0xB6),
    (Mnemonic::Ldx, AddrMode::Absolute, 0xAE),
    (Mnemonic::Ldx, AddrMode::AbsoluteY, 0xBE),
    // LDY
    (Mnemonic::Ldy, AddrMode::Immediate, 0xA0),
    (Mnemonic::Ldy, AddrMode::ZeroPage, 0xA4),
    (Mnemonic::Ldy, AddrMode::ZeroPageX, 0xB4),
    (Mnemonic::Ldy, AddrMode::Absolute, 0xAC),
    (Mnemonic::Ldy, AddrMode::AbsoluteX, 0xBC),
    // LSR
    (Mnemonic::Lsr, AddrMode::Accumulator, 0x4A),
    (Mnemonic::Lsr, AddrMode::ZeroPage, 0x46),
    (Mnemonic::Lsr, AddrMode::ZeroPageX, 0x56),
    (Mnemonic::Lsr, AddrMode::Absolute, 0x4E),
    (Mnemonic::Lsr, AddrMode::AbsoluteX, 0x5E),
    // NOP
    (Mnemonic::Nop, AddrMode::Implied, 0xEA),
    // ORA
    (Mnemonic::Ora, AddrMode::Immediate, 0x09),
    (Mnemonic::Ora, AddrMode::ZeroPage, 0x05),
    (Mnemonic::Ora, AddrMode::ZeroPageX, 0x15),
    (Mnemonic::Ora, AddrMode::Absolute, 0x0D),
    (Mnemonic::Ora, AddrMode::AbsoluteX, 0x1D), // file says 0x10 (typo); c64-wiki opcode matrix confirms 0x1D (0x10 is BPL)
    (Mnemonic::Ora, AddrMode::AbsoluteY, 0x19),
    (Mnemonic::Ora, AddrMode::IndexedIndirect, 0x01),
    (Mnemonic::Ora, AddrMode::IndirectIndexed, 0x11),
    // PHA
    (Mnemonic::Pha, AddrMode::Implied, 0x48),
    // PHP
    (Mnemonic::Php, AddrMode::Implied, 0x08),
    // PLA
    (Mnemonic::Pla, AddrMode::Implied, 0x68),
    // PLP
    (Mnemonic::Plp, AddrMode::Implied, 0x28),
    // ROL
    (Mnemonic::Rol, AddrMode::Accumulator, 0x2A),
    (Mnemonic::Rol, AddrMode::ZeroPage, 0x26),
    (Mnemonic::Rol, AddrMode::ZeroPageX, 0x36),
    (Mnemonic::Rol, AddrMode::Absolute, 0x2E),
    (Mnemonic::Rol, AddrMode::AbsoluteX, 0x3E),
    // ROR
    (Mnemonic::Ror, AddrMode::Accumulator, 0x6A),
    (Mnemonic::Ror, AddrMode::ZeroPage, 0x66),
    (Mnemonic::Ror, AddrMode::ZeroPageX, 0x76),
    (Mnemonic::Ror, AddrMode::Absolute, 0x6E),
    (Mnemonic::Ror, AddrMode::AbsoluteX, 0x7E),
    // RTI
    (Mnemonic::Rti, AddrMode::Implied, 0x40), // file says 0x4D (typo); c64-wiki opcode matrix confirms 0x40 (0x4D is EOR Absolute)
    // RTS
    (Mnemonic::Rts, AddrMode::Implied, 0x60),
    // SBC
    (Mnemonic::Sbc, AddrMode::Immediate, 0xE9),
    (Mnemonic::Sbc, AddrMode::ZeroPage, 0xE5),
    (Mnemonic::Sbc, AddrMode::ZeroPageX, 0xF5),
    (Mnemonic::Sbc, AddrMode::Absolute, 0xED),
    (Mnemonic::Sbc, AddrMode::AbsoluteX, 0xFD),
    (Mnemonic::Sbc, AddrMode::AbsoluteY, 0xF9),
    (Mnemonic::Sbc, AddrMode::IndexedIndirect, 0xE1),
    (Mnemonic::Sbc, AddrMode::IndirectIndexed, 0xF1),
    // SEC
    (Mnemonic::Sec, AddrMode::Implied, 0x38),
    // SED
    (Mnemonic::Sed, AddrMode::Implied, 0xF8),
    // SEI
    (Mnemonic::Sei, AddrMode::Implied, 0x78),
    // STA
    (Mnemonic::Sta, AddrMode::ZeroPage, 0x85),
    (Mnemonic::Sta, AddrMode::ZeroPageX, 0x95),
    (Mnemonic::Sta, AddrMode::Absolute, 0x8D), // file says 0x80 (typo); confirmed 0x8D by the hardware-verified bytes `8D 00 04` in tests/golden.rs (0x80 is an illegal/"Future Expansion" opcode, not STA)
    (Mnemonic::Sta, AddrMode::AbsoluteX, 0x9D), // file says 0x90 (typo); c64-wiki opcode matrix confirms 0x9D (0x90 is BCC)
    (Mnemonic::Sta, AddrMode::AbsoluteY, 0x99),
    (Mnemonic::Sta, AddrMode::IndexedIndirect, 0x81),
    (Mnemonic::Sta, AddrMode::IndirectIndexed, 0x91),
    // STX (no Absolute,Y — see brief edge-case note)
    (Mnemonic::Stx, AddrMode::ZeroPage, 0x86),
    (Mnemonic::Stx, AddrMode::ZeroPageY, 0x96),
    (Mnemonic::Stx, AddrMode::Absolute, 0x8E),
    // STY (no Absolute,X — see brief edge-case note)
    (Mnemonic::Sty, AddrMode::ZeroPage, 0x84),
    (Mnemonic::Sty, AddrMode::ZeroPageX, 0x94),
    (Mnemonic::Sty, AddrMode::Absolute, 0x8C),
    // TAX
    (Mnemonic::Tax, AddrMode::Implied, 0xAA),
    // TAY
    (Mnemonic::Tay, AddrMode::Implied, 0xA8),
    // TSX
    (Mnemonic::Tsx, AddrMode::Implied, 0xBA),
    // TXA
    (Mnemonic::Txa, AddrMode::Implied, 0x8A),
    // TXS
    (Mnemonic::Txs, AddrMode::Implied, 0x9A),
    // TYA
    (Mnemonic::Tya, AddrMode::Implied, 0x98),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mnemonics_case_insensitively() {
        assert_eq!(Mnemonic::parse("LDA"), Some(Mnemonic::Lda));
        assert_eq!(Mnemonic::parse("lda"), Some(Mnemonic::Lda));
        assert_eq!(Mnemonic::parse("LdA"), Some(Mnemonic::Lda));
        assert_eq!(Mnemonic::parse("LDZ"), None);
    }

    #[test]
    fn looks_up_known_opcodes() {
        assert_eq!(opcode(Mnemonic::Lda, AddrMode::Immediate), Some(0xA9));
        assert_eq!(opcode(Mnemonic::Lda, AddrMode::ZeroPage), Some(0xA5));
        assert_eq!(opcode(Mnemonic::Lda, AddrMode::Absolute), Some(0xAD));
        assert_eq!(opcode(Mnemonic::Sta, AddrMode::ZeroPage), Some(0x85));
        assert_eq!(opcode(Mnemonic::Sta, AddrMode::Absolute), Some(0x8D));
        assert_eq!(opcode(Mnemonic::Jsr, AddrMode::Absolute), Some(0x20));
        assert_eq!(opcode(Mnemonic::Rts, AddrMode::Implied), Some(0x60));
        assert_eq!(opcode(Mnemonic::Jmp, AddrMode::Indirect), Some(0x6C));
        assert_eq!(opcode(Mnemonic::Ldx, AddrMode::ZeroPageY), Some(0xB6));
        assert_eq!(opcode(Mnemonic::Stx, AddrMode::ZeroPageY), Some(0x96));
        assert_eq!(opcode(Mnemonic::Bne, AddrMode::Relative), Some(0xD0));
    }

    #[test]
    fn rejects_modes_a_mnemonic_does_not_support() {
        // JMP has no zero-page form — this is why JMP never enters the sizing loop.
        assert_eq!(opcode(Mnemonic::Jmp, AddrMode::ZeroPage), None);
        // STX supports zero page,Y but NOT absolute,Y.
        assert_eq!(opcode(Mnemonic::Stx, AddrMode::AbsoluteY), None);
        assert_eq!(opcode(Mnemonic::Stx, AddrMode::ZeroPageY), Some(0x96));
        // LDA has no accumulator form.
        assert_eq!(opcode(Mnemonic::Lda, AddrMode::Accumulator), None);
    }

    #[test]
    fn instruction_lengths_follow_addressing_mode() {
        assert_eq!(AddrMode::Implied.instr_len(), 1);
        assert_eq!(AddrMode::Accumulator.instr_len(), 1);
        assert_eq!(AddrMode::Immediate.instr_len(), 2);
        assert_eq!(AddrMode::ZeroPage.instr_len(), 2);
        assert_eq!(AddrMode::Relative.instr_len(), 2);
        assert_eq!(AddrMode::IndirectIndexed.instr_len(), 2);
        assert_eq!(AddrMode::Absolute.instr_len(), 3);
        assert_eq!(AddrMode::AbsoluteX.instr_len(), 3);
        assert_eq!(AddrMode::Indirect.instr_len(), 3);
    }

    #[test]
    fn identifies_branch_mnemonics() {
        assert!(Mnemonic::Bne.is_branch());
        assert!(Mnemonic::Bcs.is_branch());
        assert!(!Mnemonic::Jmp.is_branch());
        assert!(!Mnemonic::Lda.is_branch());
    }

    #[test]
    fn table_has_no_duplicate_entries() {
        let mut seen = std::collections::HashSet::new();
        for (m, mode, _) in OPCODES {
            assert!(
                seen.insert((*m, *mode)),
                "duplicate entry for {m:?} {mode:?}"
            );
        }
    }

    #[test]
    fn table_is_complete() {
        // The documented NMOS 6502/6510 instruction set has 151 legal opcodes.
        assert_eq!(OPCODES.len(), 151);
    }

    #[test]
    fn all_opcode_bytes_are_distinct() {
        // `table_has_no_duplicate_entries` guards (Mnemonic, AddrMode) key
        // uniqueness but says nothing about the byte column: a copy/paste or
        // transcription slip could give two different (mnemonic, mode) pairs
        // the same byte, which would make the assembler emit one instruction
        // when the source named another, with no other detector in the repo.
        let mut seen = std::collections::HashSet::new();
        for (m, mode, op) in OPCODES {
            assert!(
                seen.insert(*op),
                "opcode byte {op:#04X} used more than once (last: {m:?} {mode:?})"
            );
        }
        assert_eq!(seen.len(), 151);
    }
}
