//! Byte-to-glyph mappings for the memory dump's character column.
//!
//! Three different code systems share the range 0-255 on a C64, and the same
//! byte means a different character in each. `$41` is `A` in ASCII and in
//! PETSCII, but the screen code for `A` is `$01` — `$41` in screen RAM is a
//! graphic. Guessing which system a dumped byte belongs to is not possible
//! from the byte, and not possible from the address either: screen RAM is
//! movable and what any address exposes depends on the machine's banking
//! state. So the mode is the user's choice, passed in explicitly.
//!
//! Sources for the two C64 tables, both transcribed from the *Commodore 64
//! Programmer's Reference Guide*:
//!
//! - Screen codes — Appendix B, "Screen Display Codes", SET 1 column.
//! - PETSCII — Appendix C, "ASCII and CHR$ Codes", the `PRINT CHR$(X)` column.
//!
//! Both tables are reproduced for the **uppercase/graphics character set**
//! (SET 1), which is what the machine powers on with. Switching the C64 to
//! lowercase (SET 2) changes what several of these codes draw on the real
//! screen; QuickMon does not track that state and does not try to.

use std::fmt;

/// Shown for any byte the dump cannot render honestly: C64 graphic glyphs,
/// reverse-video codes, and control codes.
///
/// This is deliberately the same `.` the ASCII column has always used for
/// unprintable bytes. It is ambiguous with a literal `.` (`$2E`), as it is in
/// every hex dumper — the hex column beside it is the unambiguous reading.
pub const PLACEHOLDER: char = '.';

/// How the dump's character column interprets each byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CharacterMode {
    /// Printable host ASCII, `$20..=$7E`. The historical behaviour.
    #[default]
    Ascii,
    /// PETSCII, as `PRINT CHR$(X)` draws it in the uppercase/graphics set.
    Petscii,
    /// Screen (display) codes, as stored in screen RAM and read by `PEEK`.
    ScreenCodes,
}

impl CharacterMode {
    /// Every mode, in the order the selector offers them.
    pub const ALL: [CharacterMode; 3] = [
        CharacterMode::Ascii,
        CharacterMode::Petscii,
        CharacterMode::ScreenCodes,
    ];

    /// The glyph for one byte under this mode.
    ///
    /// Always exactly one `char`, so the character column is the same width
    /// in every mode and the columns stay aligned.
    pub fn glyph(self, byte: u8) -> char {
        match self {
            CharacterMode::Ascii => ascii(byte),
            CharacterMode::Petscii => petscii(byte),
            CharacterMode::ScreenCodes => screen_code(byte),
        }
    }
}

impl fmt::Display for CharacterMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            CharacterMode::Ascii => "ASCII",
            CharacterMode::Petscii => "PETSCII",
            CharacterMode::ScreenCodes => "Screen codes",
        })
    }
}

fn ascii(byte: u8) -> char {
    if (0x20..=0x7E).contains(&byte) {
        byte as char
    } else {
        PLACEHOLDER
    }
}

/// PETSCII in the uppercase/graphics set (PRG Appendix C).
///
/// `$20..=$5A` — space through `Z` — is identical to ASCII, so those bytes
/// pass through. The five codes above it are Commodore's, not ASCII's. `$A0`
/// is the shifted space, which draws as a blank.
///
/// Everything else is a control code (`$00-$1F`, `$80-$9F`) or a graphic
/// (`$60-$7F`, `$A1-$FF`), and gets the placeholder.
fn petscii(byte: u8) -> char {
    match byte {
        0x20..=0x5A => byte as char,
        0x5B => '[',
        0x5C => '£',
        0x5D => ']',
        0x5E => '↑',
        0x5F => '←',
        0xA0 => ' ',
        _ => PLACEHOLDER,
    }
}

/// Screen display codes in the uppercase/graphics set (PRG Appendix B, SET 1).
///
/// Note how far this diverges from PETSCII at the bottom of the range: the
/// letters start at `$01`, not `$41`, and `$00` is `@` rather than a null.
/// `$20..=$3F` — space through `?` — happens to coincide with ASCII.
///
/// `$60` is the shifted space, another blank. `$40-$5F` and `$61-$7F` are
/// graphics, and `$80-$FF` is the reverse-video image of `$00-$7F`. QuickMon's
/// dump cannot draw reverse video, and rendering `$81` as a plain `A` would
/// misreport what is on the screen, so the whole upper half takes the
/// placeholder.
fn screen_code(byte: u8) -> char {
    match byte {
        0x00 => '@',
        0x01..=0x1A => (b'A' + (byte - 1)) as char,
        0x1B => '[',
        0x1C => '£',
        0x1D => ']',
        0x1E => '↑',
        0x1F => '←',
        0x20..=0x3F => byte as char,
        0x60 => ' ',
        _ => PLACEHOLDER,
    }
}

#[cfg(test)]
mod tests {
    use super::CharacterMode::{Ascii, Petscii, ScreenCodes};
    use super::*;

    /// The whole reason this module exists: the same byte is a different
    /// character in each system, and `A` is the case that bites people.
    #[test]
    fn the_letter_a_lives_at_a_different_byte_in_each_system() {
        assert_eq!(Ascii.glyph(0x41), 'A');
        assert_eq!(Petscii.glyph(0x41), 'A');
        assert_eq!(ScreenCodes.glyph(0x01), 'A');

        // ...and the byte that means `A` in one system means something else
        // in the other.
        assert_eq!(ScreenCodes.glyph(0x41), PLACEHOLDER, "$41 is a graphic");
        assert_eq!(Ascii.glyph(0x01), PLACEHOLDER, "$01 is a control code");
        assert_eq!(Petscii.glyph(0x01), PLACEHOLDER, "$01 is a control code");
    }

    #[test]
    fn letters_span_their_full_range_in_each_system() {
        for (i, expected) in (b'A'..=b'Z').enumerate() {
            let expected = expected as char;
            assert_eq!(Ascii.glyph(0x41 + i as u8), expected);
            assert_eq!(Petscii.glyph(0x41 + i as u8), expected);
            assert_eq!(ScreenCodes.glyph(0x01 + i as u8), expected);
        }
    }

    /// Space, digits and punctuation coincide across all three systems, which
    /// is exactly why the letters are so easy to get wrong.
    #[test]
    fn space_digits_and_punctuation_agree_across_all_three_systems() {
        let cases = [
            (0x20, ' '),
            (0x21, '!'),
            (0x24, '$'),
            (0x2E, '.'),
            (0x30, '0'),
            (0x39, '9'),
            (0x3A, ':'),
            (0x3F, '?'),
        ];
        for (byte, expected) in cases {
            for mode in CharacterMode::ALL {
                assert_eq!(mode.glyph(byte), expected, "{mode} byte ${byte:02X}");
            }
        }
    }

    /// `@` is the one punctuation-adjacent code the two C64 systems disagree
    /// on: `$40` in PETSCII, `$00` in screen codes.
    #[test]
    fn the_at_sign_differs_between_petscii_and_screen_codes() {
        assert_eq!(Petscii.glyph(0x40), '@');
        assert_eq!(ScreenCodes.glyph(0x00), '@');
        assert_eq!(ScreenCodes.glyph(0x40), PLACEHOLDER);
    }

    #[test]
    fn commodore_specific_glyphs_are_mapped_in_both_c64_systems() {
        let cases = [('[', 0x5B, 0x1B), ('£', 0x5C, 0x1C), (']', 0x5D, 0x1D)];
        for (glyph, petscii_byte, screen_byte) in cases {
            assert_eq!(Petscii.glyph(petscii_byte), glyph);
            assert_eq!(ScreenCodes.glyph(screen_byte), glyph);
        }
        assert_eq!(Petscii.glyph(0x5E), '↑');
        assert_eq!(ScreenCodes.glyph(0x1E), '↑');
        assert_eq!(Petscii.glyph(0x5F), '←');
        assert_eq!(ScreenCodes.glyph(0x1F), '←');
    }

    /// PETSCII control codes are the bytes you would find embedded in a
    /// string constant destined for `PRINT`. None of them draw a character.
    #[test]
    fn petscii_control_codes_are_not_rendered_as_text() {
        let controls = [
            (0x05, "white"),
            (0x0D, "return"),
            (0x11, "cursor down"),
            (0x12, "reverse on"),
            (0x13, "home"),
            (0x1C, "red"),
            (0x90, "black"),
            (0x91, "cursor up"),
            (0x93, "clear"),
            (0x9D, "cursor left"),
        ];
        for (byte, name) in controls {
            assert_eq!(
                Petscii.glyph(byte),
                PLACEHOLDER,
                "${byte:02X} ({name}) is a control code, not a glyph"
            );
        }
    }

    /// Both C64 systems have a second, shifted space that draws as a blank.
    #[test]
    fn the_shifted_space_renders_as_a_blank_in_both_c64_systems() {
        assert_eq!(Petscii.glyph(0xA0), ' ');
        assert_eq!(ScreenCodes.glyph(0x60), ' ');
        assert_eq!(Ascii.glyph(0xA0), PLACEHOLDER);
    }

    /// Reverse video is the upper half of the screen-code range. The dump
    /// cannot draw it, and must not quietly show the un-reversed letter.
    #[test]
    fn reverse_video_screen_codes_are_not_shown_as_their_plain_letters() {
        assert_eq!(ScreenCodes.glyph(0x01), 'A');
        assert_eq!(
            ScreenCodes.glyph(0x81),
            PLACEHOLDER,
            "$81 is a reversed A, not a plain A"
        );
        assert_eq!(ScreenCodes.glyph(0xA0), PLACEHOLDER, "reversed space");
    }

    /// The C64 graphic glyphs have no honest monospace rendering, so both C64
    /// modes fall back to the placeholder.
    ///
    /// ASCII is deliberately excluded: several of these bytes are ordinary
    /// printable ASCII (`$66` is `f`), and ASCII mode is supposed to say so.
    /// That divergence is the feature — it is what makes the mode selector
    /// worth having.
    #[test]
    fn c64_graphics_are_not_rendered_in_either_c64_mode() {
        // Graphics in both systems: PETSCII $60-$7F and $A1-$FF,
        // screen codes $40-$5F, $61-$7F and the reversed $80-$FF.
        for byte in [0x66u8, 0x70, 0x7E, 0xA5, 0xC1, 0xDB] {
            for mode in [Petscii, ScreenCodes] {
                assert_eq!(mode.glyph(byte), PLACEHOLDER, "{mode} byte ${byte:02X}");
            }
        }
        assert_eq!(Ascii.glyph(0x66), 'f');
        assert_eq!(Ascii.glyph(0x7E), '~');
        assert_eq!(Ascii.glyph(0xA5), PLACEHOLDER);
    }

    /// Column alignment depends on this: one byte in, exactly one `char` out,
    /// for every byte and every mode.
    #[test]
    fn every_byte_maps_to_exactly_one_char_in_every_mode() {
        for mode in CharacterMode::ALL {
            for byte in 0..=u8::MAX {
                let s = mode.glyph(byte).to_string();
                assert_eq!(s.chars().count(), 1, "{mode} byte ${byte:02X}");
            }
        }
    }

    #[test]
    fn ascii_mode_still_covers_exactly_the_printable_range() {
        for byte in 0..=u8::MAX {
            let expected = (0x20..=0x7E).contains(&byte);
            assert_eq!(
                Ascii.glyph(byte) != PLACEHOLDER || byte == b'.',
                expected,
                "byte ${byte:02X}"
            );
        }
    }

    #[test]
    fn modes_are_labelled_for_the_selector() {
        assert_eq!(Ascii.to_string(), "ASCII");
        assert_eq!(Petscii.to_string(), "PETSCII");
        assert_eq!(ScreenCodes.to_string(), "Screen codes");
    }
}
