/// The API accepts a readmem length of 1..=65536.
pub fn parse_length(s: &str) -> Option<u32> {
    let v: u32 = s.trim().parse().ok()?;
    (1..=65536).contains(&v).then_some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_lengths_within_the_documented_range() {
        assert_eq!(parse_length("1"), Some(1));
        assert_eq!(parse_length("256"), Some(256));
        assert_eq!(parse_length("1000"), Some(1000));
        assert_eq!(parse_length("65536"), Some(65536));
        assert_eq!(parse_length("  512  "), Some(512));
    }

    #[test]
    fn rejects_zero_and_oversized_lengths() {
        assert_eq!(parse_length("0"), None);
        assert_eq!(parse_length("65537"), None);
        assert_eq!(parse_length(""), None);
        assert_eq!(parse_length("abc"), None);
        assert_eq!(parse_length("-1"), None);
    }
}
