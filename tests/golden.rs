use quickmon::asm::{AsmError, assemble};

/// The bytes below were produced by dasm and verified running on a real C64
/// (VICE x64sc) before this project existed: the program clears the screen,
/// writes "HI" into screen RAM at $0400/$0401, sets both colour cells white,
/// and returns. The routine contains no labels or branches, so it is
/// position-independent and these bytes are correct at any origin.
const HI_SOURCE: &str = "\
LDA #$93        ; PETSCII clear screen
JSR $FFD2       ; CHROUT
LDA #$08        ; screen code for 'H'
STA $0400
LDA #$09        ; screen code for 'I'
STA $0401
LDA #$01        ; white
STA $D800
STA $D801
LDA #$0D        ; carriage return
JSR $FFD2
RTS";

const HI_BYTES: &[u8] = &[
    0xA9, 0x93, 0x20, 0xD2, 0xFF, 0xA9, 0x08, 0x8D, 0x00, 0x04, 0xA9, 0x09, 0x8D, 0x01, 0x04, 0xA9,
    0x01, 0x8D, 0x00, 0xD8, 0x8D, 0x01, 0xD8, 0xA9, 0x0D, 0x20, 0xD2, 0xFF, 0x60,
];

#[test]
fn hi_program_matches_hardware_verified_bytes() {
    let out = assemble(HI_SOURCE, 0xC000).expect("should assemble");
    assert_eq!(out.bytes, HI_BYTES);
    assert_eq!(out.bytes.len(), 29);
}

#[test]
fn hi_program_is_position_independent() {
    let a = assemble(HI_SOURCE, 0xC000).unwrap();
    let b = assemble(HI_SOURCE, 0x0801).unwrap();
    assert_eq!(
        a.bytes, b.bytes,
        "no labels or branches, so origin must not matter"
    );
}

#[test]
fn hi_program_listing_addresses_start_at_the_origin() {
    let a = assemble(HI_SOURCE, 0xC000).unwrap();
    assert_eq!(a.lines[0].address, 0xC000);
    assert_eq!(a.lines[1].address, 0xC002);
    assert_eq!(a.lines.last().unwrap().address, 0xC000 + 28);
}

#[test]
fn parse_errors_propagate_with_line_numbers() {
    let errs: Vec<AsmError> = assemble("LDA #$08\nLDZ #$01", 0xC000).unwrap_err();
    assert_eq!(errs[0].line, 2);
}
