use crate::asm::error::AsmError;
use crate::asm::opcodes::Mnemonic;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Literal(u16),
    Symbol(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Width {
    Auto,
    Byte,
    Word,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    None,
    Accumulator,
    Immediate(Expr),
    Direct(Expr),
    DirectX(Expr),
    DirectY(Expr),
    IndexedIndirect(Expr),
    IndirectIndexed(Expr),
    Indirect(Expr),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    Instruction {
        mnemonic: Mnemonic,
        width: Width,
        operand: Operand,
    },
    Byte(Vec<Expr>),
    Word(Vec<Expr>),
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub number: usize,
    pub label: Option<String>,
    pub stmt: Stmt,
}

pub fn parse(src: &str) -> Result<Vec<Line>, Vec<AsmError>> {
    let mut lines = Vec::new();
    let mut errors = Vec::new();

    for (idx, raw) in src.lines().enumerate() {
        let number = idx + 1;
        match parse_line(raw, number) {
            Ok(line) => lines.push(line),
            Err(e) => errors.push(e),
        }
    }

    if errors.is_empty() {
        Ok(lines)
    } else {
        Err(errors)
    }
}

fn parse_line(raw: &str, number: usize) -> Result<Line, AsmError> {
    // Strip comment, then trim.
    let text = raw.split(';').next().unwrap_or("").trim();
    if text.is_empty() {
        return Ok(Line {
            number,
            label: None,
            stmt: Stmt::Empty,
        });
    }

    let (label, rest) = split_label(text);
    let rest = rest.trim();
    if rest.is_empty() {
        return Ok(Line {
            number,
            label,
            stmt: Stmt::Empty,
        });
    }

    let mut parts = rest.splitn(2, char::is_whitespace);
    let head = parts.next().unwrap_or("");
    let tail = parts.next().unwrap_or("").trim();

    if let Some(dir) = head.strip_prefix('.') {
        let lower = dir.to_ascii_lowercase();
        if lower == "byte" || lower == "word" {
            let mut exprs = Vec::new();
            for piece in tail.split(',') {
                let piece = piece.trim();
                if piece.is_empty() {
                    return Err(AsmError::new(
                        number,
                        format!("empty value in .{lower} list"),
                    ));
                }
                exprs.push(parse_expr(piece, number)?);
            }
            if exprs.is_empty() {
                return Err(AsmError::new(
                    number,
                    format!(".{lower} needs at least one value"),
                ));
            }
            let stmt = if lower == "byte" {
                Stmt::Byte(exprs)
            } else {
                Stmt::Word(exprs)
            };
            return Ok(Line {
                number,
                label,
                stmt,
            });
        }
    }

    let (mnemonic_text, width) = split_width_suffix(head);
    let mnemonic = Mnemonic::parse(mnemonic_text)
        .ok_or_else(|| AsmError::new(number, format!("unknown mnemonic '{mnemonic_text}'")))?;

    let operand = parse_operand(tail, mnemonic, number)?;
    Ok(Line {
        number,
        label,
        stmt: Stmt::Instruction {
            mnemonic,
            width,
            operand,
        },
    })
}

fn split_label(text: &str) -> (Option<String>, &str) {
    // "loop: RTS" or "loop RTS" — a leading token is a label when it is not a
    // known mnemonic and not a directive.
    let mut parts = text.splitn(2, char::is_whitespace);
    let first = parts.next().unwrap_or("");
    let rest = parts.next().unwrap_or("");

    if let Some(name) = first.strip_suffix(':') {
        return (Some(name.to_string()), rest);
    }
    if first.starts_with('.') {
        return (None, text);
    }
    let (base, _) = split_width_suffix(first);
    if Mnemonic::parse(base).is_some() {
        return (None, text);
    }
    // `first` isn't a recognized mnemonic. Treat it as a label (without a
    // colon) only when the remainder of the line looks like a real
    // statement — a known mnemonic or a `.directive`. Otherwise `first`
    // itself is the (invalid) mnemonic, and the error should point at it
    // rather than silently swallowing it as a label.
    let rest_trimmed = rest.trim();
    let rest_head = rest_trimmed.split_whitespace().next().unwrap_or("");
    let (rest_base, _) = split_width_suffix(rest_head);
    let rest_looks_like_statement = !rest_trimmed.is_empty()
        && (rest_head.starts_with('.') || Mnemonic::parse(rest_base).is_some());
    if rest_looks_like_statement && is_identifier(first) {
        return (Some(first.to_string()), rest);
    }
    (None, text)
}

fn split_width_suffix(head: &str) -> (&str, Width) {
    if let Some(base) = head.strip_suffix(".b").or_else(|| head.strip_suffix(".B")) {
        return (base, Width::Byte);
    }
    if let Some(base) = head.strip_suffix(".w").or_else(|| head.strip_suffix(".W")) {
        return (base, Width::Word);
    }
    (head, Width::Auto)
}

fn is_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn parse_operand(tail: &str, mnemonic: Mnemonic, number: usize) -> Result<Operand, AsmError> {
    let t = tail.trim();

    if t.is_empty() {
        return Ok(if is_shift(mnemonic) {
            Operand::Accumulator
        } else {
            Operand::None
        });
    }
    if t.eq_ignore_ascii_case("a") && is_shift(mnemonic) {
        return Ok(Operand::Accumulator);
    }
    if let Some(v) = t.strip_prefix('#') {
        return Ok(Operand::Immediate(parse_expr(v.trim(), number)?));
    }

    if let Some(inner) = t.strip_prefix('(') {
        if let Some(body) = inner.strip_suffix(')') {
            // ( ... ) or ( ...,X )
            if let Some(base) = strip_index_suffix(body, 'X') {
                return Ok(Operand::IndexedIndirect(parse_expr(base, number)?));
            }
            return Ok(Operand::Indirect(parse_expr(body.trim(), number)?));
        }
        if let Some(base) = strip_index_suffix(inner, 'Y') {
            // ( ... ),Y  — after stripping '(' the body still ends with ')'
            let base = base.trim().strip_suffix(')').ok_or_else(|| {
                AsmError::new(number, format!("malformed indirect operand '{t}'"))
            })?;
            return Ok(Operand::IndirectIndexed(parse_expr(base.trim(), number)?));
        }
        return Err(AsmError::new(
            number,
            format!("malformed indirect operand '{t}'"),
        ));
    }

    if let Some(base) = strip_index_suffix(t, 'X') {
        return Ok(Operand::DirectX(parse_expr(base, number)?));
    }
    if let Some(base) = strip_index_suffix(t, 'Y') {
        return Ok(Operand::DirectY(parse_expr(base, number)?));
    }
    Ok(Operand::Direct(parse_expr(t, number)?))
}

fn is_shift(m: Mnemonic) -> bool {
    matches!(
        m,
        Mnemonic::Asl | Mnemonic::Lsr | Mnemonic::Rol | Mnemonic::Ror
    )
}

/// Strips a trailing `,X` / `,Y` (case-insensitive), returning the base operand.
fn strip_index_suffix(s: &str, index: char) -> Option<&str> {
    let s = s.trim_end();
    let want_lower = index.to_ascii_lowercase();
    let last = s.chars().last()?;
    if last.to_ascii_lowercase() != want_lower {
        return None;
    }
    let without = s[..s.len() - last.len_utf8()].trim_end();
    let base = without.strip_suffix(',')?;
    Some(base.trim())
}

fn parse_expr(s: &str, number: usize) -> Result<Expr, AsmError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(AsmError::new(number, "missing operand"));
    }
    let (radix, digits) = if let Some(d) = s.strip_prefix('$') {
        (16, d)
    } else if let Some(d) = s.strip_prefix('%') {
        (2, d)
    } else if s.chars().all(|c| c.is_ascii_digit()) {
        (10, s)
    } else if is_identifier(s) {
        return Ok(Expr::Symbol(s.to_string()));
    } else {
        return Err(AsmError::new(number, format!("malformed operand '{s}'")));
    };

    if digits.is_empty() {
        return Err(AsmError::new(number, format!("malformed operand '{s}'")));
    }
    match u32::from_str_radix(digits, radix) {
        Ok(v) if v <= u16::MAX as u32 => Ok(Expr::Literal(v as u16)),
        Ok(_) => Err(AsmError::new(
            number,
            format!("value '{s}' out of range (max $FFFF)"),
        )),
        Err(_) => Err(AsmError::new(number, format!("malformed operand '{s}'"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asm::opcodes::Mnemonic;

    fn one(src: &str) -> Line {
        let mut lines = parse(src).expect("should parse");
        assert_eq!(lines.len(), 1, "expected exactly one line");
        lines.remove(0)
    }

    fn stmt(src: &str) -> Stmt {
        one(src).stmt
    }

    #[test]
    fn parses_implied() {
        assert_eq!(
            stmt("RTS"),
            Stmt::Instruction {
                mnemonic: Mnemonic::Rts,
                width: Width::Auto,
                operand: Operand::None
            }
        );
    }

    #[test]
    fn parses_accumulator_forms() {
        assert_eq!(
            stmt("ASL A"),
            Stmt::Instruction {
                mnemonic: Mnemonic::Asl,
                width: Width::Auto,
                operand: Operand::Accumulator
            }
        );
        // Bare ASL with no operand is also accumulator mode.
        assert_eq!(
            stmt("ASL"),
            Stmt::Instruction {
                mnemonic: Mnemonic::Asl,
                width: Width::Auto,
                operand: Operand::Accumulator
            }
        );
    }

    #[test]
    fn parses_immediate() {
        assert_eq!(
            stmt("LDA #$08"),
            Stmt::Instruction {
                mnemonic: Mnemonic::Lda,
                width: Width::Auto,
                operand: Operand::Immediate(Expr::Literal(0x08))
            }
        );
    }

    #[test]
    fn parses_all_indexed_and_indirect_forms() {
        assert_eq!(
            stmt("LDA $10"),
            Stmt::Instruction {
                mnemonic: Mnemonic::Lda,
                width: Width::Auto,
                operand: Operand::Direct(Expr::Literal(0x10))
            }
        );
        assert_eq!(
            stmt("LDA $1234"),
            Stmt::Instruction {
                mnemonic: Mnemonic::Lda,
                width: Width::Auto,
                operand: Operand::Direct(Expr::Literal(0x1234))
            }
        );
        assert_eq!(
            stmt("LDA $10,X"),
            Stmt::Instruction {
                mnemonic: Mnemonic::Lda,
                width: Width::Auto,
                operand: Operand::DirectX(Expr::Literal(0x10))
            }
        );
        assert_eq!(
            stmt("LDX $10,Y"),
            Stmt::Instruction {
                mnemonic: Mnemonic::Ldx,
                width: Width::Auto,
                operand: Operand::DirectY(Expr::Literal(0x10))
            }
        );
        assert_eq!(
            stmt("LDA ($10,X)"),
            Stmt::Instruction {
                mnemonic: Mnemonic::Lda,
                width: Width::Auto,
                operand: Operand::IndexedIndirect(Expr::Literal(0x10))
            }
        );
        assert_eq!(
            stmt("LDA ($10),Y"),
            Stmt::Instruction {
                mnemonic: Mnemonic::Lda,
                width: Width::Auto,
                operand: Operand::IndirectIndexed(Expr::Literal(0x10))
            }
        );
        assert_eq!(
            stmt("JMP ($1234)"),
            Stmt::Instruction {
                mnemonic: Mnemonic::Jmp,
                width: Width::Auto,
                operand: Operand::Indirect(Expr::Literal(0x1234))
            }
        );
    }

    #[test]
    fn parses_literal_radixes() {
        assert_eq!(
            stmt("LDA #$0D"),
            Stmt::Instruction {
                mnemonic: Mnemonic::Lda,
                width: Width::Auto,
                operand: Operand::Immediate(Expr::Literal(13))
            }
        );
        assert_eq!(
            stmt("LDA #13"),
            Stmt::Instruction {
                mnemonic: Mnemonic::Lda,
                width: Width::Auto,
                operand: Operand::Immediate(Expr::Literal(13))
            }
        );
        assert_eq!(
            stmt("LDA #%00001101"),
            Stmt::Instruction {
                mnemonic: Mnemonic::Lda,
                width: Width::Auto,
                operand: Operand::Immediate(Expr::Literal(13))
            }
        );
    }

    #[test]
    fn parses_width_suffixes() {
        assert_eq!(
            stmt("LDA.b $10"),
            Stmt::Instruction {
                mnemonic: Mnemonic::Lda,
                width: Width::Byte,
                operand: Operand::Direct(Expr::Literal(0x10))
            }
        );
        assert_eq!(
            stmt("LDA.w $10"),
            Stmt::Instruction {
                mnemonic: Mnemonic::Lda,
                width: Width::Word,
                operand: Operand::Direct(Expr::Literal(0x10))
            }
        );
        assert_eq!(
            stmt("lda.W $10"),
            Stmt::Instruction {
                mnemonic: Mnemonic::Lda,
                width: Width::Word,
                operand: Operand::Direct(Expr::Literal(0x10))
            }
        );
    }

    #[test]
    fn parses_symbol_operands() {
        assert_eq!(
            stmt("JMP loop"),
            Stmt::Instruction {
                mnemonic: Mnemonic::Jmp,
                width: Width::Auto,
                operand: Operand::Direct(Expr::Symbol("loop".into()))
            }
        );
        assert_eq!(
            stmt("BNE loop"),
            Stmt::Instruction {
                mnemonic: Mnemonic::Bne,
                width: Width::Auto,
                operand: Operand::Direct(Expr::Symbol("loop".into()))
            }
        );
    }

    #[test]
    fn parses_labels_with_and_without_colon() {
        assert_eq!(one("loop: RTS").label, Some("loop".to_string()));
        assert_eq!(one("loop RTS").label, Some("loop".to_string()));
        let bare = one("loop:");
        assert_eq!(bare.label, Some("loop".to_string()));
        assert_eq!(bare.stmt, Stmt::Empty);
    }

    #[test]
    fn strips_comments_and_blank_lines() {
        let lines = parse("; header comment\n\nRTS ; trailing\n").unwrap();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].stmt, Stmt::Empty);
        assert_eq!(lines[1].stmt, Stmt::Empty);
        assert_eq!(
            lines[2].stmt,
            Stmt::Instruction {
                mnemonic: Mnemonic::Rts,
                width: Width::Auto,
                operand: Operand::None
            }
        );
    }

    #[test]
    fn line_numbers_are_one_based_and_cover_every_source_line() {
        let lines = parse("RTS\nRTS\nRTS").unwrap();
        assert_eq!(
            lines.iter().map(|l| l.number).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
    }

    #[test]
    fn parses_byte_and_word_directives() {
        assert_eq!(
            stmt(".byte $01,$02,$03"),
            Stmt::Byte(vec![Expr::Literal(1), Expr::Literal(2), Expr::Literal(3)])
        );
        assert_eq!(stmt(".word $1234"), Stmt::Word(vec![Expr::Literal(0x1234)]));
        assert_eq!(
            stmt(".byte start"),
            Stmt::Byte(vec![Expr::Symbol("start".into())])
        );
    }

    #[test]
    fn reports_unknown_mnemonic_with_line_number() {
        let errs = parse("RTS\nLDZ #$01\n").unwrap_err();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].line, 2);
        assert!(
            errs[0].message.contains("LDZ"),
            "message was: {}",
            errs[0].message
        );
    }

    #[test]
    fn reports_every_bad_line_not_just_the_first() {
        let errs = parse("LDZ #$01\nLDQ #$02\n").unwrap_err();
        assert_eq!(errs.len(), 2);
        assert_eq!(errs[0].line, 1);
        assert_eq!(errs[1].line, 2);
    }

    #[test]
    fn reports_literal_out_of_range() {
        let errs = parse("LDA #$12345").unwrap_err();
        assert_eq!(errs[0].line, 1);
        assert!(
            errs[0].message.contains("range"),
            "message was: {}",
            errs[0].message
        );
    }

    #[test]
    fn reports_malformed_operand() {
        let errs = parse("LDA ($10,Y)").unwrap_err();
        assert_eq!(errs[0].line, 1);
    }
}
