//! Unescaping code
//!
//! Provides a function for converting Rust-style escaping of strings to the corresponding characters
//! so that for example a string like
//! > This isn\'t\nna\u{ef}ve
//! and outputting it as
//! > This isn't
//! > naïve

use std::borrow::Cow;
use thiserror::Error;

/// Replace escape codes with their input equivalents.
///
/// # Examples
/// ```
/// # use finl_charsub::unescape::unescape;
///
/// # fn main() -> anyhow::Result<()> {
/// assert_eq!("\t", unescape("\\t")?);
/// assert_eq!("\r", unescape("\\r")?);
/// assert_eq!("\n", unescape("\\n")?);
/// assert_eq!("'", unescape("\\'")?);
/// assert_eq!("\"", unescape("\\\"")?);
/// assert_eq!("\\", unescape("\\\\")?);
/// assert_eq!("\u{a0}", unescape("\\u{a0}")?);
/// # Ok(())
/// # }
/// ```
pub fn unescape<'a>(input: &'a str) -> anyhow::Result<Cow<'a, str>> {
    let mut state = State::Normal;
    let mut escape_sequence_seen = false;
    // unescaping is guaranteed to have a length ≤ the input length
    let mut modified_string = String::with_capacity(input.len());
    let mut unicode_value = 0u32;

    for c in input.char_indices() {
        match state {
            State::Normal => {
                match c {
                    (index, '\\') => {
                        if !escape_sequence_seen {
                            if index > 0 {
                                // if unwrap panics, something is wrong with this code
                                modified_string.push_str(input.get(0..index).unwrap());
                            }
                        }
                        escape_sequence_seen = true;
                        state = State::Escape;
                    }
                    (_, c) => {
                        if escape_sequence_seen {
                            modified_string.push(c);
                        }
                    }
                }
            }
            State::Escape => match c {
                (_, 't') => modified_string.push('\t'),
                (_, '\\') => modified_string.push('\\'),
                (_, '"') => modified_string.push('"'),
                (_, '\'') => modified_string.push('\''),
                (_, 'n') => modified_string.push('\n'),
                (_, 'r') => modified_string.push('\r'),
                (_, 'u') => state = State::StartUnicode,
                (index, ch) => {
                    anyhow::bail!(UnescapeError::BadEscape(
                        input.get(..index).unwrap().to_string(),
                        ch
                    ));
                }
            },

            State::StartUnicode => {
                if c.1 != '{' {
                    anyhow::bail!(UnescapeError::MissingOpenBrace(
                        input.get(..c.0).unwrap().to_string(),
                        c.1
                    ));
                }
                unicode_value = 0;
                state = State::InUnicode;
            }

            State::InUnicode => match c {
                (index, '}') => {
                    let possible_char = char::from_u32(unicode_value);
                    match possible_char {
                        None => {
                            anyhow::bail!(UnescapeError::InvalidUnicodeValue(
                                input.get(..index).unwrap().to_string(),
                                '}'
                            ));
                        }
                        Some(valid_char) => {
                            modified_string.push(valid_char);
                            state = State::Normal;
                        }
                    }
                }
                (index, ch) => {
                    let digit = ch.to_digit(0x10);
                    match digit {
                        None => {
                            anyhow::bail!(UnescapeError::NonHexDigit(
                                input.get(..index).unwrap().to_string(),
                                ch
                            ));
                        }
                        Some(d) => {
                            unicode_value = (unicode_value << 4) + d;
                            if unicode_value > 0x10FFFF {
                                anyhow::bail!(UnescapeError::HexValueTooLarge(
                                    input.get(..index).unwrap().to_string(),
                                    ch
                                ))
                            }
                        }
                    }
                }
            },
        }
    }

    if escape_sequence_seen {
        Ok(modified_string.into())
    } else {
        Ok(input.into())
    }
}

/// Errors returned when parsing. Each variant has a `String` containing the successfully parsed
/// portion of the input
/// and `char` which which markes the point where parsing failed.
#[derive(Error, Debug)]
pub enum UnescapeError {
    /// Given when there's an unrecognized character after the `\` escape.
    #[error("Bad escape found. Failed at: {0}{1}")]
    BadEscape(String, char),
    /// Given when `\u` is not followed by `{`
    #[error("Missing open brace after \\u. Failed at: {0}{1}")]
    MissingOpenBrace(String, char),
    /// Given if there is a character which is not a hex digit in the braces following `\u`
    #[error("Non-hex digit in \\u. Failed at: {0}{1}")]
    NonHexDigit(String, char),
    /// Given when the value in the braces following `\u` exceeds `0x10FFFF`
    #[error("Hex value too large in \\u. Failed at: {0}{1}")]
    HexValueTooLarge(String, char),
    /// Given when the value in the braces following `\u` is not a valid Unicode character code.
    #[error("Invalid value in \\u. Failed at: {0}{1}")]
    InvalidUnicodeValue(String, char),
}

enum State {
    Normal,
    Escape,
    StartUnicode,
    InUnicode,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_string_comes_back_the_same() -> anyhow::Result<()> {
        assert_eq!("ordinary", unescape("ordinary")?);
        Ok(())
    }

    #[test]
    fn tab_escape_is_decoded() -> anyhow::Result<()> {
        assert_eq!("\t", unescape("\\t")?);
        Ok(())
    }
    #[test]
    fn r_escape_is_decoded() -> anyhow::Result<()> {
        assert_eq!("\r", unescape("\\r")?);
        Ok(())
    }
    #[test]
    fn n_escape_is_decoded() -> anyhow::Result<()> {
        assert_eq!("\n", unescape("\\n")?);
        Ok(())
    }
    #[test]
    fn single_quote_escape_is_decoded() -> anyhow::Result<()> {
        assert_eq!("'", unescape("\\'")?);
        Ok(())
    }
    #[test]
    fn double_quote_escape_is_decoded() -> anyhow::Result<()> {
        assert_eq!("\"", unescape("\\\"")?);
        Ok(())
    }
    #[test]
    fn backslash_escape_is_decoded() -> anyhow::Result<()> {
        assert_eq!("\\", unescape("\\\\")?);
        Ok(())
    }

    #[test]
    fn bad_escape_gives_error() {
        let result = unescape("foo \\0");
        assert_eq!(true, result.is_err());
        assert_eq!(
            "Bad escape found. Failed at: foo \\0",
            format!("{}", result.err().unwrap())
        )
    }

    #[test]
    fn missing_brace_after_u_gives_error() {
        let result = unescape("foo \\un");
        assert_eq!(true, result.is_err());
        assert_eq!(
            "Missing open brace after \\u. Failed at: foo \\un",
            format!("{}", result.err().unwrap())
        )
    }

    #[test]
    fn non_hex_digit_after_u_gives_error() {
        let result = unescape("foo \\u{n}");
        assert_eq!(true, result.is_err());
        assert_eq!(
            "Non-hex digit in \\u. Failed at: foo \\u{n",
            format!("{}", result.err().unwrap())
        )
    }

    #[test]
    fn too_many_hex_digits_after_u_gives_error() {
        let result = unescape("foo \\u{1000000}");
        assert_eq!(true, result.is_err());
        assert_eq!(
            "Hex value too large in \\u. Failed at: foo \\u{1000000",
            format!("{}", result.err().unwrap())
        )
    }

    #[test]
    fn too_large_a_value_gives_error() {
        let result = unescape("foo \\u{120000}");
        assert_eq!(true, result.is_err());
        assert_eq!(
            "Hex value too large in \\u. Failed at: foo \\u{120000",
            format!("{}", result.err().unwrap())
        )
    }

    #[test]
    fn invalid_code_point_gives_error() {
        let result = unescape("foo \\u{d800}");
        assert_eq!(true, result.is_err());
        assert_eq!(
            "Invalid value in \\u. Failed at: foo \\u{d800}",
            format!("{}", result.err().unwrap())
        )
    }

    #[test]
    fn unicode_escape_is_decoded() -> anyhow::Result<()> {
        assert_eq!("a\u{a0}b", unescape("a\\u{a0}b")?);
        Ok(())
    }
}
