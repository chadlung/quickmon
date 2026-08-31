use crate::ui::charmap::CharacterMode;

pub fn parse_addr(s: &str) -> Option<u16> {
    let t = s.trim();
    let digits = t
        .strip_prefix('$')
        .or_else(|| t.strip_prefix("0x"))
        .or_else(|| t.strip_prefix("0X"))
        .unwrap_or(t);
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(digits, 16)
        .ok()
        .filter(|v| *v <= 0xFFFF)
        .map(|v| v as u16)
}

pub fn format_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Format `bytes` as 16-byte rows: address, hex, then a character column
/// interpreted under `mode`.
///
/// `mode` affects only the character column. The address and hex columns are
/// the same bytes however they are being read.
pub fn hex_dump(address: u16, bytes: &[u8], mode: CharacterMode) -> Vec<String> {
    bytes
        .chunks(16)
        .enumerate()
        .map(|(row, chunk)| {
            let addr = address.wrapping_add((row * 16) as u16);
            let hex = chunk
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(" ");
            let chars: String = chunk.iter().map(|b| mode.glyph(*b)).collect();
            format!("{addr:04X}  {hex:<47}  |{chars}|")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_address_in_common_forms() {
        assert_eq!(parse_addr("C000"), Some(0xC000));
        assert_eq!(parse_addr("$C000"), Some(0xC000));
        assert_eq!(parse_addr("0xC000"), Some(0xC000));
        assert_eq!(parse_addr("c000"), Some(0xC000));
        assert_eq!(parse_addr("  C000  "), Some(0xC000));
        assert_eq!(parse_addr("400"), Some(0x0400));
    }

    #[test]
    fn rejects_bad_addresses() {
        assert_eq!(parse_addr(""), None);
        assert_eq!(parse_addr("$"), None);
        assert_eq!(parse_addr("GHIJ"), None);
        assert_eq!(parse_addr("1FFFF"), None); // > $FFFF
    }

    #[test]
    fn formats_bytes_as_uppercase_hex() {
        assert_eq!(format_bytes(&[0xA9, 0x08, 0x0d]), "A9 08 0D");
        assert_eq!(format_bytes(&[]), "");
    }

    #[test]
    fn hex_dump_rows_are_sixteen_bytes_wide() {
        let data: Vec<u8> = (0..20).collect();
        let rows = hex_dump(0x0400, &data, CharacterMode::Ascii);
        assert_eq!(rows.len(), 2);
        assert!(rows[0].starts_with("0400  "), "row was: {}", rows[0]);
        assert!(rows[1].starts_with("0410  "), "row was: {}", rows[1]);
    }

    #[test]
    fn hex_dump_renders_printable_ascii_and_dots() {
        let rows = hex_dump(0x0400, b"HI\x00\xff", CharacterMode::Ascii);
        assert!(rows[0].contains("48 49 00 FF"), "row was: {}", rows[0]);
        assert!(rows[0].contains("|HI..|"), "row was: {}", rows[0]);
    }

    #[test]
    fn hex_dump_of_empty_input_is_empty() {
        assert!(hex_dump(0x0400, &[], CharacterMode::Ascii).is_empty());
    }

    /// The same bytes read three ways. `$08 $09` is `HI` in screen RAM, and
    /// `$48 $49` is `HI` to `PRINT` — each mode picks out its own.
    #[test]
    fn the_character_column_follows_the_selected_mode() {
        let bytes = [0x48, 0x49, 0x08, 0x09];
        let col = |mode| {
            let row = hex_dump(0x0400, &bytes, mode).remove(0);
            row[row.find('|').unwrap()..].to_string()
        };
        assert_eq!(col(CharacterMode::Ascii), "|HI..|");
        assert_eq!(col(CharacterMode::Petscii), "|HI..|");
        assert_eq!(col(CharacterMode::ScreenCodes), "|..HI|");
    }

    /// Changing how the bytes are *read* must never change what the dump says
    /// the bytes *are*.
    #[test]
    fn addresses_and_hex_are_identical_in_every_mode() {
        let data: Vec<u8> = (0..=u8::MAX).collect();

        let columns = |mode| -> Vec<String> {
            hex_dump(0xC000, &data, mode)
                .iter()
                .map(|row| row[..row.find('|').unwrap()].to_string())
                .collect()
        };

        let baseline = columns(CharacterMode::Ascii);
        assert_eq!(baseline.len(), 16);
        for mode in CharacterMode::ALL {
            assert_eq!(columns(mode), baseline, "{mode} changed the hex column");
        }
    }

    /// Graphics bytes take the placeholder in every mode, and the hex beside
    /// them still reports the real value.
    #[test]
    fn unrepresentable_graphics_use_the_placeholder_without_touching_the_hex() {
        for mode in CharacterMode::ALL {
            let rows = hex_dump(0x0400, &[0xA5, 0xDB], mode);
            assert!(rows[0].contains("A5 DB"), "{mode} row was: {}", rows[0]);
            assert!(rows[0].ends_with("|..|"), "{mode} row was: {}", rows[0]);
        }
    }

    /// Every row is the same width in every mode, so the columns line up no
    /// matter what is selected.
    #[test]
    fn rows_have_a_stable_width_across_modes() {
        let data: Vec<u8> = (0..=u8::MAX).collect();
        for mode in CharacterMode::ALL {
            for row in hex_dump(0x0400, &data, mode) {
                assert_eq!(row.chars().count(), 4 + 2 + 47 + 2 + 1 + 16 + 1);
            }
        }
    }
}
