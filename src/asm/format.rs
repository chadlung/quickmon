//! Source tidying applied when the user presses Assemble.

use crate::asm::opcodes::Mnemonic;

/// Uppercases instruction mnemonics, leaving everything else exactly as typed.
///
/// Deliberately narrow. The tempting one-liner — uppercase the whole line — is
/// wrong three ways in this assembler:
///
/// * **Labels are case-sensitive.** `loop` and `LOOP` are different symbols, so
///   rewriting a definition or a reference can silently repoint or break a jump.
/// * **Comments are prose.** Shouting the user's own notes back at them is not
///   a formatting improvement.
/// * **A label may spell a mnemonic.** `JMP lda` references a label named
///   `lda`; uppercasing that operand would break the reference.
///
/// So only the token in *mnemonic position* is touched: the first token of the
/// statement, after an optional label. A `.b` / `.w` width suffix keeps its
/// documented lowercase spelling — `lda.w` becomes `LDA.w`.
///
/// Directives (`.byte`, `.word`) are left alone: they are not instructions, and
/// lowercase is their conventional spelling.
pub fn uppercase_mnemonics(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    for (i, line) in src.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&uppercase_line(line));
    }
    out
}

fn uppercase_line(line: &str) -> String {
    // Never touch a comment.
    let code_len = line.find(';').unwrap_or(line.len());
    let (code, comment) = line.split_at(code_len);

    let tokens = tokens(code);
    let target = match tokens.as_slice() {
        // Bare instruction, or an instruction with operands.
        [first, ..] if is_mnemonic(&code[first.0..first.1]) => Some(*first),
        // Label then instruction, with or without the colon.
        [first, second, ..]
            if code[first.0..first.1].ends_with(':') || !is_mnemonic(&code[first.0..first.1]) =>
        {
            is_mnemonic(&code[second.0..second.1]).then_some(*second)
        }
        _ => None,
    };

    let Some((start, end)) = target else {
        return line.to_string();
    };

    // Uppercase the mnemonic but keep any `.b` / `.w` suffix as typed.
    let token = &code[start..end];
    let (base, suffix) = split_suffix(token);
    let mut s = String::with_capacity(line.len());
    s.push_str(&code[..start]);
    s.push_str(&base.to_ascii_uppercase());
    s.push_str(suffix);
    s.push_str(&code[end..]);
    s.push_str(comment);
    s
}

/// Byte ranges of whitespace-separated tokens, so the original spacing survives.
fn tokens(code: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in code.char_indices() {
        match (c.is_whitespace(), start) {
            (false, None) => start = Some(i),
            (true, Some(s)) => {
                out.push((s, i));
                start = None;
            }
            _ => {}
        }
        if out.len() == 2 {
            break;
        }
    }
    if let Some(s) = start
        && out.len() < 2
    {
        out.push((s, code.len()));
    }
    out
}

fn split_suffix(token: &str) -> (&str, &str) {
    for suffix in [".b", ".B", ".w", ".W"] {
        if let Some(base) = token.strip_suffix(suffix) {
            return (base, &token[base.len()..]);
        }
    }
    (token, "")
}

fn is_mnemonic(token: &str) -> bool {
    let (base, _) = split_suffix(token);
    Mnemonic::parse(base).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn up(s: &str) -> String {
        uppercase_mnemonics(s)
    }

    #[test]
    fn uppercases_a_bare_instruction() {
        assert_eq!(up("rts"), "RTS");
        assert_eq!(up("lda #$08"), "LDA #$08");
        assert_eq!(up("Sta $0400"), "STA $0400");
    }

    #[test]
    fn preserves_indentation_and_inner_spacing() {
        assert_eq!(up("    lda   #$08"), "    LDA   #$08");
        assert_eq!(up("\tjsr\t$ffd2"), "\tJSR\t$ffd2");
    }

    #[test]
    fn leaves_operands_alone() {
        // Hex digits and label references keep the case the user typed.
        assert_eq!(up("jsr $ffd2"), "JSR $ffd2");
        assert_eq!(up("jmp done"), "JMP done");
    }

    #[test]
    fn handles_a_label_before_the_instruction() {
        assert_eq!(up("loop: lda $10"), "loop: LDA $10");
        assert_eq!(up("loop lda $10"), "loop LDA $10");
        assert_eq!(up("Loop:  bne  Loop"), "Loop:  BNE  Loop");
    }

    #[test]
    fn never_touches_a_label_that_spells_a_mnemonic() {
        // Labels are case-sensitive: rewriting either end of this pair would
        // break the reference or repoint it at a different symbol.
        assert_eq!(up("jmp lda"), "JMP lda");
        assert_eq!(up("lda: rts"), "lda: RTS");
    }

    #[test]
    fn never_touches_comments() {
        assert_eq!(up("; lda is a load"), "; lda is a load");
        assert_eq!(up("lda #$08 ; load the sta value"), "LDA #$08 ; load the sta value");
        assert_eq!(up("  ; leading blank then prose"), "  ; leading blank then prose");
    }

    #[test]
    fn keeps_the_width_suffix_lowercase() {
        assert_eq!(up("lda.w $10"), "LDA.w $10");
        assert_eq!(up("sta.b $10"), "STA.b $10");
    }

    #[test]
    fn leaves_directives_alone() {
        assert_eq!(up(".byte $01,$02"), ".byte $01,$02");
        assert_eq!(up("data: .word $1234"), "data: .word $1234");
    }

    #[test]
    fn leaves_unknown_mnemonics_alone() {
        // If it will not assemble, do not disguise it as valid.
        assert_eq!(up("ldz #$01"), "ldz #$01");
    }

    #[test]
    fn preserves_line_structure_exactly() {
        assert_eq!(up(""), "");
        assert_eq!(up("\n"), "\n");
        assert_eq!(up("rts\n"), "RTS\n");
        assert_eq!(up("lda #$08\n\nrts\n"), "LDA #$08\n\nRTS\n");
        // No trailing newline is invented and none is eaten.
        assert_eq!(up("lda #$08\nrts"), "LDA #$08\nRTS");
    }

    #[test]
    fn tab_indentation_survives_and_assembles() {
        // Tab is what the editor inserts, so it has to work end to end: kept
        // byte-for-byte by the formatter, and producing the same machine code
        // as the space-indented equivalent.
        let tabbed = "loop:\n\tlda #$08\n\tbne loop\n";
        assert_eq!(
            uppercase_mnemonics(tabbed),
            "loop:\n\tLDA #$08\n\tBNE loop\n",
            "tabs must be preserved exactly"
        );
        let spaced = "loop:\n    lda #$08\n    bne loop\n";
        assert_eq!(
            crate::asm::assemble(tabbed, 0xC000).unwrap().bytes,
            crate::asm::assemble(spaced, 0xC000).unwrap().bytes,
            "tab- and space-indented source must assemble identically"
        );
    }

    #[test]
    fn is_idempotent() {
        let once = up("loop: lda.w $10 ; keep this\nbne loop\n");
        assert_eq!(up(&once), once);
    }

    #[test]
    fn does_not_change_what_the_program_assembles_to() {
        let lower = "lda #$93\njsr $ffd2\nrts";
        let a = crate::asm::assemble(lower, 0xC000).unwrap();
        let b = crate::asm::assemble(&up(lower), 0xC000).unwrap();
        assert_eq!(a.bytes, b.bytes, "tidying must not alter the machine code");
    }
}
