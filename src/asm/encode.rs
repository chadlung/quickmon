use std::collections::BTreeMap;

use crate::asm::error::AsmError;
use crate::asm::opcodes::{AddrMode, Mnemonic, opcode};
use crate::asm::parser::{Expr, Line, Operand, Stmt, Width};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingLine {
    pub address: u16,
    pub start: usize,
    pub len: usize,
    pub source_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assembly {
    pub org: u16,
    pub bytes: Vec<u8>,
    pub lines: Vec<ListingLine>,
    pub symbols: BTreeMap<String, u16>,
}

/// Which addressing modes an operand form could legally use, in order from
/// narrowest to widest. Only forms with two candidates enter the sizing loop.
fn candidates(operand: &Operand) -> &'static [AddrMode] {
    use AddrMode::*;
    match operand {
        Operand::None => &[Implied],
        Operand::Accumulator => &[Accumulator],
        Operand::Immediate(_) => &[Immediate],
        Operand::Direct(_) => &[ZeroPage, Absolute],
        Operand::DirectX(_) => &[ZeroPageX, AbsoluteX],
        Operand::DirectY(_) => &[ZeroPageY, AbsoluteY],
        Operand::IndexedIndirect(_) => &[IndexedIndirect],
        Operand::IndirectIndexed(_) => &[IndirectIndexed],
        Operand::Indirect(_) => &[Indirect],
    }
}

fn operand_expr(operand: &Operand) -> Option<&Expr> {
    match operand {
        Operand::None | Operand::Accumulator => None,
        Operand::Immediate(e)
        | Operand::Direct(e)
        | Operand::DirectX(e)
        | Operand::DirectY(e)
        | Operand::IndexedIndirect(e)
        | Operand::IndirectIndexed(e)
        | Operand::Indirect(e) => Some(e),
    }
}

/// The modes a given mnemonic actually supports, filtered from the candidates.
/// Human name for an operand form, for error messages.
fn operand_form(operand: &Operand) -> &'static str {
    match operand {
        Operand::None => "no operand",
        Operand::Accumulator => "an accumulator operand",
        Operand::Immediate(_) => "an immediate operand",
        Operand::Direct(_) => "a direct operand",
        Operand::DirectX(_) => "an ,X-indexed operand",
        Operand::DirectY(_) => "a ,Y-indexed operand",
        Operand::IndexedIndirect(_) => "an (indirect,X) operand",
        Operand::IndirectIndexed(_) => "an (indirect),Y operand",
        Operand::Indirect(_) => "an indirect operand",
    }
}

fn legal_modes(m: Mnemonic, operand: &Operand) -> Vec<AddrMode> {
    if m.is_branch() {
        // A branch takes exactly one operand form: a direct target, encoded
        // relative. Every other form is an error — including no operand at
        // all, which previously reserved two bytes during layout and emitted
        // only the one-byte opcode. That left the emitted program one byte
        // short of what every later label's address assumed: the bytes and
        // the symbol table disagreed, silently.
        return match operand {
            Operand::Direct(_) => vec![AddrMode::Relative],
            _ => Vec::new(),
        };
    }
    candidates(operand)
        .iter()
        .copied()
        .filter(|mode| opcode(m, *mode).is_some())
        .collect()
}

pub fn encode(lines: &[Line], org: u16) -> Result<Assembly, Vec<AsmError>> {
    // Index of every instruction statement, with its currently chosen mode.
    let mut chosen: Vec<Option<AddrMode>> = Vec::with_capacity(lines.len());
    let mut errors: Vec<AsmError> = Vec::new();

    // Initial choice: narrowest legal mode, honouring an explicit width suffix.
    for line in lines {
        let mode = match &line.stmt {
            Stmt::Instruction {
                mnemonic,
                width,
                operand,
            } => {
                let legal = legal_modes(*mnemonic, operand);
                if legal.is_empty() {
                    errors.push(AsmError::new(
                        line.number,
                        format!(
                            "{} does not accept {}",
                            name_of(*mnemonic),
                            operand_form(operand)
                        ),
                    ));
                    None
                } else {
                    match pick_initial(&legal, *width) {
                        Ok(initial) => Some(initial),
                        Err(class) => {
                            errors.push(AsmError::new(
                                line.number,
                                format!(
                                    "{} with {} has no {} form, so {} cannot be honoured",
                                    name_of(*mnemonic),
                                    operand_form(operand),
                                    class.description(),
                                    class.suffix()
                                ),
                            ));
                            None
                        }
                    }
                }
            }
            _ => None,
        };
        chosen.push(mode);
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    // Fixpoint: lay out, resolve symbols, widen anything that needs it, repeat.
    // Widths only grow, so this terminates in at most `lines.len()` iterations.
    let mut symbols;
    loop {
        symbols = match layout(lines, &chosen, org) {
            Ok(s) => s,
            Err(e) => return Err(e),
        };

        let mut widened = false;
        for (i, line) in lines.iter().enumerate() {
            let Stmt::Instruction {
                mnemonic,
                width,
                operand,
            } = &line.stmt
            else {
                continue;
            };
            if *width != Width::Auto {
                continue; // explicit override never widens
            }
            let legal = legal_modes(*mnemonic, operand);
            if legal.len() < 2 {
                continue; // unambiguous
            }
            let Some(expr) = operand_expr(operand) else {
                continue;
            };
            let Some(value) = resolve(expr, &symbols) else {
                continue;
            };
            let current = chosen[i].expect("instruction has a mode");
            if value > 0xFF && current.operand_len() == 1 && !mnemonic.is_branch() {
                chosen[i] = Some(legal[legal.len() - 1]);
                widened = true;
            }
        }

        if !widened {
            break;
        }
    }

    emit(lines, &chosen, org, &symbols)
}

/// The class of addressing a width suffix selects.
///
/// `.b` and `.w` name an *address width*, not merely an operand byte count.
/// Immediate, implied, accumulator, relative, and both indirect-indexed forms
/// carry no address, so neither suffix applies to them — previously both were
/// silently ignored there, and `LDA.b #$10`, `BNE.w target` and `RTS.w` all
/// assembled as though the suffix had not been written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WidthClass {
    Byte,
    Word,
}

impl WidthClass {
    fn description(self) -> &'static str {
        match self {
            WidthClass::Byte => "zero page",
            WidthClass::Word => "absolute",
        }
    }

    fn suffix(self) -> &'static str {
        match self {
            WidthClass::Byte => ".b",
            WidthClass::Word => ".w",
        }
    }

    fn accepts(self, mode: AddrMode) -> bool {
        use AddrMode::*;
        match self {
            WidthClass::Byte => matches!(mode, ZeroPage | ZeroPageX | ZeroPageY),
            WidthClass::Word => matches!(mode, Absolute | AbsoluteX | AbsoluteY | Indirect),
        }
    }
}

/// Picks the starting mode. Without a suffix this is the narrowest legal mode,
/// which the fixpoint loop may later widen. With a suffix it must be a mode of
/// the requested class, or the suffix cannot be honoured and assembly fails.
fn pick_initial(legal: &[AddrMode], width: Width) -> Result<AddrMode, WidthClass> {
    let class = match width {
        Width::Auto => return Ok(legal[0]),
        Width::Byte => WidthClass::Byte,
        Width::Word => WidthClass::Word,
    };
    legal
        .iter()
        .copied()
        .find(|m| class.accepts(*m))
        .ok_or(class)
}

fn stmt_len(line: &Line, mode: Option<AddrMode>) -> usize {
    match &line.stmt {
        Stmt::Instruction { .. } => mode.map(AddrMode::instr_len).unwrap_or(0),
        Stmt::Byte(v) => v.len(),
        Stmt::Word(v) => v.len() * 2,
        Stmt::Empty => 0,
    }
}

fn layout(
    lines: &[Line],
    chosen: &[Option<AddrMode>],
    org: u16,
) -> Result<BTreeMap<String, u16>, Vec<AsmError>> {
    let mut symbols = BTreeMap::new();
    let mut errors = Vec::new();
    let mut pc = org as usize;

    for (i, line) in lines.iter().enumerate() {
        if let Some(label) = &line.label {
            if symbols.insert(label.clone(), pc as u16).is_some() {
                errors.push(AsmError::new(
                    line.number,
                    format!("duplicate label '{label}'"),
                ));
            }
        }
        pc += stmt_len(line, chosen[i]);
        if pc > 0x1_0000 {
            errors.push(AsmError::new(line.number, "program runs past $FFFF"));
            break;
        }
    }

    if errors.is_empty() {
        Ok(symbols)
    } else {
        Err(errors)
    }
}

fn resolve(expr: &Expr, symbols: &BTreeMap<String, u16>) -> Option<u16> {
    match expr {
        Expr::Literal(v) => Some(*v),
        Expr::Symbol(name) => symbols.get(name).copied(),
    }
}

fn emit(
    lines: &[Line],
    chosen: &[Option<AddrMode>],
    org: u16,
    symbols: &BTreeMap<String, u16>,
) -> Result<Assembly, Vec<AsmError>> {
    let mut bytes: Vec<u8> = Vec::new();
    let mut listing = Vec::new();
    let mut errors = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let start = bytes.len();
        let address = org as usize + start;

        match &line.stmt {
            Stmt::Empty => continue,

            Stmt::Byte(exprs) => {
                for e in exprs {
                    match resolve(e, symbols) {
                        None => errors.push(undefined(line.number, e)),
                        Some(v) if v > 0xFF => errors.push(AsmError::new(
                            line.number,
                            format!(".byte value ${v:04X} does not fit in one byte"),
                        )),
                        Some(v) => bytes.push(v as u8),
                    }
                }
            }

            Stmt::Word(exprs) => {
                for e in exprs {
                    match resolve(e, symbols) {
                        None => errors.push(undefined(line.number, e)),
                        Some(v) => {
                            bytes.push((v & 0xFF) as u8);
                            bytes.push((v >> 8) as u8);
                        }
                    }
                }
            }

            Stmt::Instruction {
                mnemonic,
                width,
                operand,
            } => {
                let mode = chosen[i].expect("instruction has a mode");
                let op = match opcode(*mnemonic, mode) {
                    Some(op) => op,
                    None => {
                        errors.push(AsmError::new(
                            line.number,
                            format!(
                                "{} does not support this addressing mode",
                                name_of(*mnemonic)
                            ),
                        ));
                        continue;
                    }
                };
                bytes.push(op);

                if let Some(expr) = operand_expr(operand) {
                    let Some(value) = resolve(expr, symbols) else {
                        errors.push(undefined(line.number, expr));
                        continue;
                    };

                    if mode == AddrMode::Relative {
                        // The 6510 adds the signed displacement to the 16-bit
                        // address of the *next* instruction and wraps at 16
                        // bits, so a branch across $FFFF/$0000 is legal
                        // hardware. Subtracting as i32 modelled a flat address
                        // space and rejected those, reporting nonsense like
                        // "-65536 bytes".
                        //
                        // Reinterpreting the wrapping u16 difference as i16 is
                        // not a narrowing cast — both are 16 bits — so the
                        // -128..=127 check below remains a real bound and a
                        // distant target cannot masquerade as reachable.
                        let next = (address as u16).wrapping_add(2);
                        let disp = value.wrapping_sub(next) as i16;
                        if !(-128..=127).contains(&disp) {
                            errors.push(AsmError::new(
                                line.number,
                                format!(
                                    "branch to '{}' is {disp} bytes, limit is -128 to +127",
                                    describe(expr)
                                ),
                            ));
                            continue;
                        }
                        bytes.push(disp as i8 as u8);
                    } else if mode.operand_len() == 1 {
                        if value > 0xFF {
                            let name = name_of(*mnemonic);
                            errors.push(AsmError::new(
                                line.number,
                                format!(
                                    "{name} operand ${value:04X} does not fit in a zero page operand{}",
                                    if *width == Width::Byte {
                                        " (.b forces one byte)".to_string()
                                    } else {
                                        format!(" and {name} has no wider form for it")
                                    }
                                ),
                            ));
                            continue;
                        }
                        bytes.push(value as u8);
                    } else {
                        bytes.push((value & 0xFF) as u8);
                        bytes.push((value >> 8) as u8);
                    }
                }
            }
        }

        let len = bytes.len() - start;
        if len > 0 {
            listing.push(ListingLine {
                address: address as u16,
                start,
                len,
                source_line: line.number,
            });
        }
    }

    if org as usize + bytes.len() > 0x1_0000 {
        errors.push(AsmError::new(
            lines.last().map(|l| l.number).unwrap_or(1),
            "program runs past $FFFF",
        ));
    }

    if errors.is_empty() {
        Ok(Assembly {
            org,
            bytes,
            lines: listing,
            symbols: symbols.clone(),
        })
    } else {
        Err(errors)
    }
}

fn undefined(line: usize, expr: &Expr) -> AsmError {
    AsmError::new(line, format!("undefined symbol '{}'", describe(expr)))
}

fn describe(expr: &Expr) -> String {
    match expr {
        Expr::Literal(v) => format!("${v:04X}"),
        Expr::Symbol(s) => s.clone(),
    }
}

/// Renders a mnemonic in its uppercase assembly spelling, so error messages say
/// "STX" rather than the Debug spelling "Stx".
fn name_of(m: Mnemonic) -> String {
    format!("{m:?}").to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asm::parser::parse;

    fn asm(src: &str, org: u16) -> Assembly {
        let lines = parse(src).expect("parse should succeed");
        encode(&lines, org).expect("encode should succeed")
    }

    fn bytes(src: &str, org: u16) -> Vec<u8> {
        asm(src, org).bytes
    }

    fn errors(src: &str, org: u16) -> Vec<AsmError> {
        let lines = parse(src).expect("parse should succeed");
        encode(&lines, org).expect_err("encode should fail")
    }

    #[test]
    fn emits_each_addressing_mode() {
        assert_eq!(bytes("RTS", 0xC000), vec![0x60]);
        assert_eq!(bytes("ASL A", 0xC000), vec![0x0A]);
        assert_eq!(bytes("LDA #$08", 0xC000), vec![0xA9, 0x08]);
        assert_eq!(bytes("LDA $10", 0xC000), vec![0xA5, 0x10]);
        assert_eq!(bytes("LDA $10,X", 0xC000), vec![0xB5, 0x10]);
        assert_eq!(bytes("LDX $10,Y", 0xC000), vec![0xB6, 0x10]);
        assert_eq!(bytes("LDA $1234", 0xC000), vec![0xAD, 0x34, 0x12]);
        assert_eq!(bytes("LDA $1234,X", 0xC000), vec![0xBD, 0x34, 0x12]);
        assert_eq!(bytes("LDA $1234,Y", 0xC000), vec![0xB9, 0x34, 0x12]);
        assert_eq!(bytes("LDA ($10,X)", 0xC000), vec![0xA1, 0x10]);
        assert_eq!(bytes("LDA ($10),Y", 0xC000), vec![0xB1, 0x10]);
        assert_eq!(bytes("JMP ($1234)", 0xC000), vec![0x6C, 0x34, 0x12]);
    }

    #[test]
    fn emits_little_endian_addresses() {
        // $0400 must serialize as 00 04, not 04 00.
        assert_eq!(bytes("STA $0400", 0xC000), vec![0x8D, 0x00, 0x04]);
        assert_eq!(bytes("STA $D800", 0xC000), vec![0x8D, 0x00, 0xD8]);
    }

    #[test]
    fn resolves_backward_label_reference() {
        assert_eq!(bytes("loop: JMP loop", 0xC000), vec![0x4C, 0x00, 0xC0]);
    }

    #[test]
    fn resolves_forward_label_reference() {
        // JMP is 3 bytes, so `done` is at $C003.
        assert_eq!(
            bytes("JMP done\ndone: RTS", 0xC000),
            vec![0x4C, 0x03, 0xC0, 0x60]
        );
    }

    // ---- the sizing loop ----

    #[test]
    fn reference_direction_does_not_change_output() {
        // A zero-page label used before its definition must assemble to the same
        // encoding as one used after. This is the regression test for the sizing loop.
        let after = bytes("target: .byte $AA\nLDA target", 0x0010);
        let before = bytes("LDA target\ntarget: .byte $AA", 0x0010);

        // `after`:  target=$0010 (zp) -> LDA zp is 2 bytes.
        assert_eq!(after, vec![0xAA, 0xA5, 0x10]);
        // `before`: LDA sized as zp (2 bytes) -> target=$0012, still zero page.
        assert_eq!(before, vec![0xA5, 0x12, 0xAA]);
        // Both used the 2-byte zero-page encoding. A naive "forward refs are
        // absolute" assembler would emit 0xAD here and be one byte longer.
        assert_eq!(before[0], 0xA5);
        assert_eq!(before.len(), 3);
    }

    #[test]
    fn widens_forward_reference_that_lands_outside_zero_page() {
        // target resolves to $C002, which does not fit in a byte -> absolute.
        let out = bytes("LDA target\ntarget: .byte $AA", 0xC000);
        assert_eq!(out, vec![0xAD, 0x03, 0xC0, 0xAA]);
    }

    #[test]
    fn sizing_loop_converges_when_one_widening_forces_another() {
        // A genuine cascade, and the reason this design needs a fixpoint rather
        // than a fixed number of passes.
        //
        // Pass 1: both LDAs assumed zero page (2 bytes each) -> y=$00FF, x=$0100.
        //         x is out of zero page, so the FIRST LDA widens to absolute.
        // Pass 2: that extra byte shifts everything down -> y=$0100, x=$0101.
        //         y has now been pushed out of zero page too, so the SECOND LDA
        //         widens. This second widening exists only because of the first.
        // Pass 3: y=$0101, x=$0102, both already absolute -> fixpoint, no widening.
        //
        // Asserted unconditionally: a guard like `if sym <= 0xFF` would make the
        // test hold vacuously no matter what the loop did.
        let src = "\
LDA x
LDA y
y: .byte $01
x: .byte $02";
        assert_eq!(
            bytes(src, 0x00FB),
            vec![0xAD, 0x02, 0x01, 0xAD, 0x01, 0x01, 0x01, 0x02]
        );

        let a = asm(src, 0x00FB);
        assert_eq!(a.symbols["y"], 0x0101);
        assert_eq!(a.symbols["x"], 0x0102);
    }

    #[test]
    fn jmp_never_uses_zero_page() {
        // JMP has no zero-page form, so a low target still assembles absolute.
        assert_eq!(
            bytes("JMP target\ntarget: RTS", 0x0010),
            vec![0x4C, 0x13, 0x00, 0x60]
        );
    }

    #[test]
    fn stx_indexed_y_has_only_one_encoding() {
        // STX supports zero page,Y but not absolute,Y.
        assert_eq!(bytes("STX $10,Y", 0xC000), vec![0x96, 0x10]);
        let errs = errors("STX $1234,Y", 0xC000);
        assert!(
            errs[0].message.to_lowercase().contains("stx"),
            "message was: {}",
            errs[0].message
        );
    }

    // ---- width overrides ----

    #[test]
    fn width_suffix_forces_encoding() {
        assert_eq!(bytes("LDA.b $10", 0xC000), vec![0xA5, 0x10]);
        assert_eq!(bytes("LDA.w $10", 0xC000), vec![0xAD, 0x10, 0x00]);
    }

    #[test]
    fn width_suffixes_select_an_address_width_or_fail() {
        // Every mandatory case from the hardening spec's section 1.3. `.b` and
        // `.w` name an address width, not an operand byte count, so they apply
        // only to zero-page and absolute modes respectively. Previously the
        // suffix was silently ignored wherever it did not fit, which is the
        // worst outcome for a feature whose entire purpose is deliberate
        // control over instruction length and cycle timing.
        assert_eq!(bytes("LDA.b $10", 0xC000), vec![0xA5, 0x10]);
        assert_eq!(bytes("LDA.w $10", 0xC000), vec![0xAD, 0x10, 0x00]);
        assert_eq!(bytes("JMP.w $10", 0xC000), vec![0x4C, 0x10, 0x00]);
        // Indirect carries a 16-bit address, so `.w` applies to it.
        assert_eq!(bytes("JMP.w ($10)", 0xC000), vec![0x6C, 0x10, 0x00]);
        assert_eq!(bytes("STX.b $10,Y", 0xC000), vec![0x96, 0x10]);

        for (src, want) in [
            // The mnemonic simply lacks that form.
            ("LDA.b $1234", "zero page"),
            ("JMP.b $10", "no zero page form"),
            ("STX.w $10,Y", "no absolute form"),
            // The operand form carries no address at all, so neither suffix
            // can apply. All four of these used to assemble silently.
            ("LDA.b #$10", "no zero page form"),
            ("BNE.b t", "no zero page form"),
            ("BNE.w t", "no absolute form"),
            ("RTS.b", "no zero page form"),
            ("RTS.w", "no absolute form"),
        ] {
            let errs = errors(src, 0xC000);
            assert_eq!(errs[0].line, 1, "{src}");
            assert!(
                errs[0].message.contains(want),
                "{src} gave: {}",
                errs[0].message
            );
        }
    }

    #[test]
    fn automatic_width_selection_is_unaffected_by_the_strict_suffix_rules() {
        // The no-suffix path must still start narrow and let the fixpoint
        // widen, including the cascade.
        assert_eq!(bytes("LDA $10", 0xC000), vec![0xA5, 0x10]);
        assert_eq!(bytes("LDA $1234", 0xC000), vec![0xAD, 0x34, 0x12]);
        assert_eq!(
            bytes("LDA x\nLDA y\ny: .byte $01\nx: .byte $02", 0x00FB),
            vec![0xAD, 0x02, 0x01, 0xAD, 0x01, 0x01, 0x01, 0x02]
        );
    }

    #[test]
    fn word_suffix_errors_when_no_wider_form_exists() {
        // STX has zero page,Y but no absolute,Y. `.w` must force absolute or fail —
        // silently emitting the 2-byte zero page form would defeat the whole point
        // of the suffix, which is deliberate control over length and cycle timing.
        let errs = errors("STX.w $10,Y", 0xC000);
        assert_eq!(errs[0].line, 1);
        assert!(
            errs[0].message.contains("STX"),
            "message was: {}",
            errs[0].message
        );
        assert!(
            errs[0].message.contains(".w"),
            "message was: {}",
            errs[0].message
        );

        // ...but the fix must not over-reach: `.w` on a mnemonic that does have an
        // absolute form still widens as before.
        assert_eq!(bytes("LDA.w $10", 0xC000), vec![0xAD, 0x10, 0x00]);
    }

    #[test]
    fn byte_suffix_on_oversized_value_is_an_error_not_a_truncation() {
        let errs = errors("LDA.b $1234", 0xC000);
        assert_eq!(errs[0].line, 1);
        assert!(
            errs[0].message.contains("$1234") || errs[0].message.contains("zero page"),
            "message was: {}",
            errs[0].message
        );
    }

    // ---- branches ----

    #[test]
    fn branches_accept_only_a_direct_target() {
        // Every one of these previously reached the relative-encoding path and
        // was rejected — if at all — by the displacement range check, which
        // produced nonsense like "branch to '$0010' is -49138 bytes".
        for src in [
            "BNE #$10",
            "BNE ($10)",
            "BNE ($10,X)",
            "BNE ($10),Y",
            "BNE $10,X",
            "BNE $10,Y",
        ] {
            let errs = errors(src, 0xC000);
            assert_eq!(errs[0].line, 1, "{src}");
            assert!(
                errs[0].message.contains("BNE does not accept"),
                "{src} gave: {}",
                errs[0].message
            );
        }
        // The one legal form still works.
        assert_eq!(bytes("BNE t\nt: RTS", 0xC000), vec![0xD0, 0x00, 0x60]);
    }

    #[test]
    fn a_branch_without_an_operand_cannot_corrupt_later_labels() {
        // This is the defect that mattered: layout reserved two bytes for the
        // branch and emission wrote one, so the emitted program was a byte
        // shorter than every later label's address assumed. `BNE\nend: RTS`
        // assembled to D0 60 with `end` recorded at $C002 while the 60 sat at
        // $C001 — wrong bytes and a lying symbol table, silently.
        let errs = errors("BNE\nend: RTS", 0xC000);
        assert_eq!(errs[0].line, 1);
        assert!(
            errs[0].message.contains("does not accept no operand"),
            "message was: {}",
            errs[0].message
        );
    }

    #[test]
    fn branches_wrap_at_the_16_bit_address_boundary() {
        // The 6510 adds the displacement to the 16-bit address of the next
        // instruction and wraps, so both of these are legal hardware. The old
        // i32 subtraction modelled a flat address space and rejected them.
        assert_eq!(bytes("BNE $0000", 0xFFFE), vec![0xD0, 0x00]);
        assert_eq!(bytes("BNE $FFFF", 0x0000), vec![0xD0, 0xFD]);
    }

    #[test]
    fn wrapping_does_not_make_distant_targets_reachable() {
        // The u16->i16 reinterpretation must not turn an unreachable target
        // into a valid short branch.
        let errs = errors("BNE $C200", 0xC000);
        assert_eq!(errs[0].line, 1);
        assert!(
            errs[0].message.contains("limit is -128 to +127"),
            "message was: {}",
            errs[0].message
        );
        // And the reported distance must be sane, not the old "-65536".
        assert!(
            !errs[0].message.contains("65536"),
            "message was: {}",
            errs[0].message
        );
    }

    #[test]
    fn branch_offsets_forward_and_backward() {
        // BNE at $C000 is 2 bytes; target $C002 -> offset 0.
        assert_eq!(bytes("BNE done\ndone: RTS", 0xC000), vec![0xD0, 0x00, 0x60]);
        // Backward to self-start: BNE at $C001, next instr $C003, target $C000 -> -3 = $FD.
        assert_eq!(bytes("top: NOP\nBNE top", 0xC000), vec![0xEA, 0xD0, 0xFD]);
    }

    #[test]
    fn branch_range_boundaries_are_accepted() {
        // +127: 127 bytes of NOP between the branch and the target.
        let mut src = String::from("BNE far\n");
        for _ in 0..127 {
            src.push_str("NOP\n");
        }
        src.push_str("far: RTS");
        let out = bytes(&src, 0xC000);
        assert_eq!(out[0], 0xD0);
        assert_eq!(out[1], 127);

        // -128
        let mut src = String::from("back: NOP\n");
        for _ in 0..125 {
            src.push_str("NOP\n");
        }
        src.push_str("BNE back");
        let out = bytes(&src, 0xC000);
        assert_eq!(*out.last().unwrap(), 0x80); // -128
    }

    #[test]
    fn branch_out_of_range_reports_displacement_and_limit() {
        let mut src = String::from("BNE far\n");
        for _ in 0..200 {
            src.push_str("NOP\n");
        }
        src.push_str("far: RTS");
        let errs = errors(&src, 0xC000);
        assert_eq!(errs[0].line, 1);
        let m = &errs[0].message;
        assert!(m.contains("far"), "message was: {m}");
        assert!(
            m.contains("127") || m.contains("128"),
            "message should state the limit: {m}"
        );
    }

    #[test]
    fn branch_displacement_accounts_for_widening_during_sizing() {
        // The LDA between the branch and its target widens to absolute because
        // `data` is high; the branch offset must reflect the final width.
        let src = "\
BNE done
LDA data
done: RTS
data: .byte $AA";
        let out = bytes(src, 0xC000);
        assert_eq!(out[0], 0xD0);
        assert_eq!(out[1], 3); // skips a 3-byte absolute LDA
        assert_eq!(out[2], 0xAD);
    }

    // ---- directives ----

    #[test]
    fn emits_byte_and_word_directives() {
        assert_eq!(bytes(".byte $01,$02,$03", 0xC000), vec![0x01, 0x02, 0x03]);
        assert_eq!(bytes(".word $1234", 0xC000), vec![0x34, 0x12]);
        assert_eq!(
            bytes("target: .byte $AA\n.word target", 0xC000),
            vec![0xAA, 0x00, 0xC0]
        );
    }

    #[test]
    fn byte_directive_rejects_oversized_value() {
        let errs = errors(".byte $1234", 0xC000);
        assert_eq!(errs[0].line, 1);
    }

    // ---- errors ----

    #[test]
    fn reports_undefined_symbol() {
        let errs = errors("JMP nowhere", 0xC000);
        assert_eq!(errs[0].line, 1);
        assert!(
            errs[0].message.contains("nowhere"),
            "message was: {}",
            errs[0].message
        );
    }

    #[test]
    fn reports_duplicate_label() {
        let errs = errors("dup: RTS\ndup: RTS", 0xC000);
        assert_eq!(errs[0].line, 2);
        assert!(
            errs[0].message.contains("dup"),
            "message was: {}",
            errs[0].message
        );
    }

    #[test]
    fn reports_mode_not_supported_by_mnemonic() {
        // Bare `LDA` parses as Operand::None, and LDA has no implied form, so this
        // reaches the "no legal mode at all" branch. (`LDA A` would NOT reach it:
        // the parser maps a bare `A` to Accumulator only for shift mnemonics, so
        // `LDA A` becomes Direct(Symbol("A")) and reports an undefined symbol
        // instead.) Assert on the message so this cannot drift to another branch.
        let errs = errors("LDA", 0xC000);
        assert_eq!(errs[0].line, 1);
        assert!(
            errs[0].message.contains("LDA"),
            "message was: {}",
            errs[0].message
        );
        assert!(
            errs[0].message.contains("does not accept no operand"),
            "message was: {}",
            errs[0].message
        );
    }

    #[test]
    fn reports_program_running_past_ffff() {
        let errs = errors(".word $1111\n.word $2222", 0xFFFF);
        assert!(!errs.is_empty());
    }

    // ---- listing ----

    #[test]
    fn listing_maps_bytes_back_to_source_lines() {
        let a = asm("LDA #$08\nSTA $0400\nRTS", 0xC000);
        assert_eq!(a.lines.len(), 3);
        assert_eq!(
            (
                a.lines[0].address,
                a.lines[0].start,
                a.lines[0].len,
                a.lines[0].source_line
            ),
            (0xC000, 0, 2, 1)
        );
        assert_eq!(
            (
                a.lines[1].address,
                a.lines[1].start,
                a.lines[1].len,
                a.lines[1].source_line
            ),
            (0xC002, 2, 3, 2)
        );
        assert_eq!(
            (
                a.lines[2].address,
                a.lines[2].start,
                a.lines[2].len,
                a.lines[2].source_line
            ),
            (0xC005, 5, 1, 3)
        );
    }

    #[test]
    fn empty_lines_produce_no_listing_entries() {
        let a = asm("; comment\n\nRTS", 0xC000);
        assert_eq!(a.lines.len(), 1);
        assert_eq!(a.lines[0].source_line, 3);
    }
}
