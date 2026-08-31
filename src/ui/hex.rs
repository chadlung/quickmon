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

pub fn hex_dump(address: u16, bytes: &[u8]) -> Vec<String> {
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
            let ascii: String = chunk
                .iter()
                .map(|b| {
                    if (0x20..0x7F).contains(b) {
                        *b as char
                    } else {
                        '.'
                    }
                })
                .collect();
            format!("{addr:04X}  {hex:<47}  |{ascii}|")
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
        let rows = hex_dump(0x0400, &data);
        assert_eq!(rows.len(), 2);
        assert!(rows[0].starts_with("0400  "), "row was: {}", rows[0]);
        assert!(rows[1].starts_with("0410  "), "row was: {}", rows[1]);
    }

    #[test]
    fn hex_dump_renders_printable_ascii_and_dots() {
        let rows = hex_dump(0x0400, b"HI\x00\xff");
        assert!(rows[0].contains("48 49 00 FF"), "row was: {}", rows[0]);
        assert!(rows[0].contains("|HI..|"), "row was: {}", rows[0]);
    }

    #[test]
    fn hex_dump_of_empty_input_is_empty() {
        assert!(hex_dump(0x0400, &[]).is_empty());
    }
}
