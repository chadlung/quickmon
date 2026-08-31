//! Independent conformance matrix for the documented MOS 6510 instruction set.
//!
//! The point of this file is to be a *second opinion*. `src/asm/opcodes.rs`
//! already tests its own table for length, key-uniqueness and byte-uniqueness,
//! but a transcription error that happens to stay unique passes all three. That
//! is not hypothetical here: the scanned reference the production table was
//! originally transcribed from carried eight OCR corruptions, one of which
//! turned `STA Absolute` into `$80` — an illegal opcode that would have made
//! `STA $0400` misbehave silently on real hardware.
//!
//! The expected values below were therefore NOT generated from `OPCODES`. They
//! were extracted from three independent published sources and cross-checked
//! against each other before being written here:
//!
//!   * <https://sta.c64.org/cbm64mcinst1.html>   (flat 0-255 listing, 151 documented)
//!   * <https://c64os.com/post/6502instructions> (per-instruction tables, 151 documented)
//!   * <https://www.c64-wiki.com/wiki/Opcode>    (16x16 matrix, incl. illegals)
//!
//! All three agreed on every one of the 151 documented entries — same bytes,
//! same mnemonics, same addressing modes. The c64-wiki matrix additionally
//! lists the 105 undocumented encodings; those are deliberately absent here,
//! since QuickMon supports the documented set only.
//!
//! If a future edit to `OPCODES` disagrees with this file, this file is the one
//! more likely to be right.

use quickmon::asm::opcodes::OPCODES;
use quickmon::asm::{AddrMode, Mnemonic};
use std::collections::{HashMap, HashSet};

/// (opcode byte, mnemonic, addressing mode) for all 151 documented instructions.
#[rustfmt::skip]
const EXPECTED: &[(u8, Mnemonic, AddrMode)] = &[
    (0x00, Mnemonic::Brk, AddrMode::Implied),
    (0x01, Mnemonic::Ora, AddrMode::IndexedIndirect),
    (0x05, Mnemonic::Ora, AddrMode::ZeroPage),
    (0x06, Mnemonic::Asl, AddrMode::ZeroPage),
    (0x08, Mnemonic::Php, AddrMode::Implied),
    (0x09, Mnemonic::Ora, AddrMode::Immediate),
    (0x0A, Mnemonic::Asl, AddrMode::Accumulator),
    (0x0D, Mnemonic::Ora, AddrMode::Absolute),
    (0x0E, Mnemonic::Asl, AddrMode::Absolute),
    (0x10, Mnemonic::Bpl, AddrMode::Relative),
    (0x11, Mnemonic::Ora, AddrMode::IndirectIndexed),
    (0x15, Mnemonic::Ora, AddrMode::ZeroPageX),
    (0x16, Mnemonic::Asl, AddrMode::ZeroPageX),
    (0x18, Mnemonic::Clc, AddrMode::Implied),
    (0x19, Mnemonic::Ora, AddrMode::AbsoluteY),
    (0x1D, Mnemonic::Ora, AddrMode::AbsoluteX),
    (0x1E, Mnemonic::Asl, AddrMode::AbsoluteX),
    (0x20, Mnemonic::Jsr, AddrMode::Absolute),
    (0x21, Mnemonic::And, AddrMode::IndexedIndirect),
    (0x24, Mnemonic::Bit, AddrMode::ZeroPage),
    (0x25, Mnemonic::And, AddrMode::ZeroPage),
    (0x26, Mnemonic::Rol, AddrMode::ZeroPage),
    (0x28, Mnemonic::Plp, AddrMode::Implied),
    (0x29, Mnemonic::And, AddrMode::Immediate),
    (0x2A, Mnemonic::Rol, AddrMode::Accumulator),
    (0x2C, Mnemonic::Bit, AddrMode::Absolute),
    (0x2D, Mnemonic::And, AddrMode::Absolute),
    (0x2E, Mnemonic::Rol, AddrMode::Absolute),
    (0x30, Mnemonic::Bmi, AddrMode::Relative),
    (0x31, Mnemonic::And, AddrMode::IndirectIndexed),
    (0x35, Mnemonic::And, AddrMode::ZeroPageX),
    (0x36, Mnemonic::Rol, AddrMode::ZeroPageX),
    (0x38, Mnemonic::Sec, AddrMode::Implied),
    (0x39, Mnemonic::And, AddrMode::AbsoluteY),
    (0x3D, Mnemonic::And, AddrMode::AbsoluteX),
    (0x3E, Mnemonic::Rol, AddrMode::AbsoluteX),
    (0x40, Mnemonic::Rti, AddrMode::Implied),
    (0x41, Mnemonic::Eor, AddrMode::IndexedIndirect),
    (0x45, Mnemonic::Eor, AddrMode::ZeroPage),
    (0x46, Mnemonic::Lsr, AddrMode::ZeroPage),
    (0x48, Mnemonic::Pha, AddrMode::Implied),
    (0x49, Mnemonic::Eor, AddrMode::Immediate),
    (0x4A, Mnemonic::Lsr, AddrMode::Accumulator),
    (0x4C, Mnemonic::Jmp, AddrMode::Absolute),
    (0x4D, Mnemonic::Eor, AddrMode::Absolute),
    (0x4E, Mnemonic::Lsr, AddrMode::Absolute),
    (0x50, Mnemonic::Bvc, AddrMode::Relative),
    (0x51, Mnemonic::Eor, AddrMode::IndirectIndexed),
    (0x55, Mnemonic::Eor, AddrMode::ZeroPageX),
    (0x56, Mnemonic::Lsr, AddrMode::ZeroPageX),
    (0x58, Mnemonic::Cli, AddrMode::Implied),
    (0x59, Mnemonic::Eor, AddrMode::AbsoluteY),
    (0x5D, Mnemonic::Eor, AddrMode::AbsoluteX),
    (0x5E, Mnemonic::Lsr, AddrMode::AbsoluteX),
    (0x60, Mnemonic::Rts, AddrMode::Implied),
    (0x61, Mnemonic::Adc, AddrMode::IndexedIndirect),
    (0x65, Mnemonic::Adc, AddrMode::ZeroPage),
    (0x66, Mnemonic::Ror, AddrMode::ZeroPage),
    (0x68, Mnemonic::Pla, AddrMode::Implied),
    (0x69, Mnemonic::Adc, AddrMode::Immediate),
    (0x6A, Mnemonic::Ror, AddrMode::Accumulator),
    (0x6C, Mnemonic::Jmp, AddrMode::Indirect),
    (0x6D, Mnemonic::Adc, AddrMode::Absolute),
    (0x6E, Mnemonic::Ror, AddrMode::Absolute),
    (0x70, Mnemonic::Bvs, AddrMode::Relative),
    (0x71, Mnemonic::Adc, AddrMode::IndirectIndexed),
    (0x75, Mnemonic::Adc, AddrMode::ZeroPageX),
    (0x76, Mnemonic::Ror, AddrMode::ZeroPageX),
    (0x78, Mnemonic::Sei, AddrMode::Implied),
    (0x79, Mnemonic::Adc, AddrMode::AbsoluteY),
    (0x7D, Mnemonic::Adc, AddrMode::AbsoluteX),
    (0x7E, Mnemonic::Ror, AddrMode::AbsoluteX),
    (0x81, Mnemonic::Sta, AddrMode::IndexedIndirect),
    (0x84, Mnemonic::Sty, AddrMode::ZeroPage),
    (0x85, Mnemonic::Sta, AddrMode::ZeroPage),
    (0x86, Mnemonic::Stx, AddrMode::ZeroPage),
    (0x88, Mnemonic::Dey, AddrMode::Implied),
    (0x8A, Mnemonic::Txa, AddrMode::Implied),
    (0x8C, Mnemonic::Sty, AddrMode::Absolute),
    (0x8D, Mnemonic::Sta, AddrMode::Absolute),
    (0x8E, Mnemonic::Stx, AddrMode::Absolute),
    (0x90, Mnemonic::Bcc, AddrMode::Relative),
    (0x91, Mnemonic::Sta, AddrMode::IndirectIndexed),
    (0x94, Mnemonic::Sty, AddrMode::ZeroPageX),
    (0x95, Mnemonic::Sta, AddrMode::ZeroPageX),
    (0x96, Mnemonic::Stx, AddrMode::ZeroPageY),
    (0x98, Mnemonic::Tya, AddrMode::Implied),
    (0x99, Mnemonic::Sta, AddrMode::AbsoluteY),
    (0x9A, Mnemonic::Txs, AddrMode::Implied),
    (0x9D, Mnemonic::Sta, AddrMode::AbsoluteX),
    (0xA0, Mnemonic::Ldy, AddrMode::Immediate),
    (0xA1, Mnemonic::Lda, AddrMode::IndexedIndirect),
    (0xA2, Mnemonic::Ldx, AddrMode::Immediate),
    (0xA4, Mnemonic::Ldy, AddrMode::ZeroPage),
    (0xA5, Mnemonic::Lda, AddrMode::ZeroPage),
    (0xA6, Mnemonic::Ldx, AddrMode::ZeroPage),
    (0xA8, Mnemonic::Tay, AddrMode::Implied),
    (0xA9, Mnemonic::Lda, AddrMode::Immediate),
    (0xAA, Mnemonic::Tax, AddrMode::Implied),
    (0xAC, Mnemonic::Ldy, AddrMode::Absolute),
    (0xAD, Mnemonic::Lda, AddrMode::Absolute),
    (0xAE, Mnemonic::Ldx, AddrMode::Absolute),
    (0xB0, Mnemonic::Bcs, AddrMode::Relative),
    (0xB1, Mnemonic::Lda, AddrMode::IndirectIndexed),
    (0xB4, Mnemonic::Ldy, AddrMode::ZeroPageX),
    (0xB5, Mnemonic::Lda, AddrMode::ZeroPageX),
    (0xB6, Mnemonic::Ldx, AddrMode::ZeroPageY),
    (0xB8, Mnemonic::Clv, AddrMode::Implied),
    (0xB9, Mnemonic::Lda, AddrMode::AbsoluteY),
    (0xBA, Mnemonic::Tsx, AddrMode::Implied),
    (0xBC, Mnemonic::Ldy, AddrMode::AbsoluteX),
    (0xBD, Mnemonic::Lda, AddrMode::AbsoluteX),
    (0xBE, Mnemonic::Ldx, AddrMode::AbsoluteY),
    (0xC0, Mnemonic::Cpy, AddrMode::Immediate),
    (0xC1, Mnemonic::Cmp, AddrMode::IndexedIndirect),
    (0xC4, Mnemonic::Cpy, AddrMode::ZeroPage),
    (0xC5, Mnemonic::Cmp, AddrMode::ZeroPage),
    (0xC6, Mnemonic::Dec, AddrMode::ZeroPage),
    (0xC8, Mnemonic::Iny, AddrMode::Implied),
    (0xC9, Mnemonic::Cmp, AddrMode::Immediate),
    (0xCA, Mnemonic::Dex, AddrMode::Implied),
    (0xCC, Mnemonic::Cpy, AddrMode::Absolute),
    (0xCD, Mnemonic::Cmp, AddrMode::Absolute),
    (0xCE, Mnemonic::Dec, AddrMode::Absolute),
    (0xD0, Mnemonic::Bne, AddrMode::Relative),
    (0xD1, Mnemonic::Cmp, AddrMode::IndirectIndexed),
    (0xD5, Mnemonic::Cmp, AddrMode::ZeroPageX),
    (0xD6, Mnemonic::Dec, AddrMode::ZeroPageX),
    (0xD8, Mnemonic::Cld, AddrMode::Implied),
    (0xD9, Mnemonic::Cmp, AddrMode::AbsoluteY),
    (0xDD, Mnemonic::Cmp, AddrMode::AbsoluteX),
    (0xDE, Mnemonic::Dec, AddrMode::AbsoluteX),
    (0xE0, Mnemonic::Cpx, AddrMode::Immediate),
    (0xE1, Mnemonic::Sbc, AddrMode::IndexedIndirect),
    (0xE4, Mnemonic::Cpx, AddrMode::ZeroPage),
    (0xE5, Mnemonic::Sbc, AddrMode::ZeroPage),
    (0xE6, Mnemonic::Inc, AddrMode::ZeroPage),
    (0xE8, Mnemonic::Inx, AddrMode::Implied),
    (0xE9, Mnemonic::Sbc, AddrMode::Immediate),
    (0xEA, Mnemonic::Nop, AddrMode::Implied),
    (0xEC, Mnemonic::Cpx, AddrMode::Absolute),
    (0xED, Mnemonic::Sbc, AddrMode::Absolute),
    (0xEE, Mnemonic::Inc, AddrMode::Absolute),
    (0xF0, Mnemonic::Beq, AddrMode::Relative),
    (0xF1, Mnemonic::Sbc, AddrMode::IndirectIndexed),
    (0xF5, Mnemonic::Sbc, AddrMode::ZeroPageX),
    (0xF6, Mnemonic::Inc, AddrMode::ZeroPageX),
    (0xF8, Mnemonic::Sed, AddrMode::Implied),
    (0xF9, Mnemonic::Sbc, AddrMode::AbsoluteY),
    (0xFD, Mnemonic::Sbc, AddrMode::AbsoluteX),
    (0xFE, Mnemonic::Inc, AddrMode::AbsoluteX),
];

#[test]
fn expected_matrix_is_internally_consistent() {
    // Guards the fixture itself: 151 entries, every byte distinct, every
    // (mnemonic, mode) pair distinct.
    assert_eq!(
        EXPECTED.len(),
        151,
        "the documented 6510 set has 151 opcodes"
    );

    let mut bytes = HashSet::new();
    let mut pairs = HashSet::new();
    for (b, m, mode) in EXPECTED {
        assert!(
            bytes.insert(*b),
            "duplicate opcode byte ${b:02X} in the fixture"
        );
        assert!(
            pairs.insert((*m, *mode)),
            "duplicate {m:?}/{mode:?} pair in the fixture"
        );
    }
}

#[test]
fn production_table_matches_the_independent_matrix() {
    let actual: HashMap<(Mnemonic, AddrMode), u8> = OPCODES
        .iter()
        .map(|(m, mode, b)| ((*m, *mode), *b))
        .collect();

    let mut wrong = Vec::new();
    let mut missing = Vec::new();
    for (b, m, mode) in EXPECTED {
        match actual.get(&(*m, *mode)) {
            None => missing.push(format!("{m:?}/{mode:?} (expected ${b:02X})")),
            Some(got) if got != b => wrong.push(format!(
                "{m:?}/{mode:?}: table has ${got:02X}, sources say ${b:02X}"
            )),
            Some(_) => {}
        }
    }
    assert!(
        wrong.is_empty(),
        "wrong opcode bytes:\n  {}",
        wrong.join("\n  ")
    );
    assert!(
        missing.is_empty(),
        "missing from OPCODES:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn production_table_has_no_entries_beyond_the_documented_set() {
    // Catches an *added* entry, which a per-expected-entry check cannot see.
    // Undocumented opcodes are out of scope for QuickMon, so anything here
    // that the three sources do not list is a defect.
    let expected: HashSet<(Mnemonic, AddrMode)> =
        EXPECTED.iter().map(|(_, m, mode)| (*m, *mode)).collect();

    let extra: Vec<String> = OPCODES
        .iter()
        .filter(|(m, mode, _)| !expected.contains(&(*m, *mode)))
        .map(|(m, mode, b)| format!("{m:?}/{mode:?} = ${b:02X}"))
        .collect();

    assert!(
        extra.is_empty(),
        "not in the documented set:\n  {}",
        extra.join("\n  ")
    );
    assert_eq!(
        OPCODES.len(),
        EXPECTED.len(),
        "table and fixture differ in length"
    );
}
