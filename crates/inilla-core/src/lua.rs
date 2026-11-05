//! Writing Rust values into Lua source, for the few places iNilla calls a stock verb by running a
//! chunk (the launcher's `DefaultServerLogin`).

/// `s` as a Lua string literal that reads back byte for byte: quotes, backslashes, control
/// characters and anything outside printable ASCII become escapes (`\ddd` is Lua 5.1's decimal
/// byte escape), so no account name or password can end the literal or add code.
pub fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for &b in s.as_bytes() {
        match b {
            b'"' => out.push_str(r#"\""#),
            b'\\' => out.push_str(r"\\"),
            0x20..=0x7e => out.push(b as char),
            _ => out.push_str(&format!(r"\{b:03}")),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::quote;

    #[test]
    fn plain_text_is_wrapped() {
        assert_eq!(quote("hunter2"), r#""hunter2""#);
    }

    #[test]
    fn quotes_and_backslashes_cannot_end_the_literal() {
        assert_eq!(quote(r#"a"b\c"#), r#""a\"b\\c""#);
        assert_eq!(quote(r#"") os.exit() --"#), r#""\") os.exit() --""#);
    }

    #[test]
    fn control_and_non_ascii_bytes_are_decimal_escapes() {
        assert_eq!(quote("a\nb"), r#""a\010b""#);
        assert_eq!(quote("]]"), r#""]]""#);
        // `é` is two UTF-8 bytes, each escaped; Lua rebuilds the same bytes.
        assert_eq!(quote("é"), r#""\195\169""#);
        // A digit after an escape cannot be read as part of it: `\ddd` is always three digits.
        assert_eq!(quote("\u{1}2"), r#""\0012""#);
    }
}
