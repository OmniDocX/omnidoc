//! XML escape helpers.

use std::borrow::Cow;
use std::fmt;

/// Escape special characters for XML text content and attribute values.
///
/// Escapes `&`, `<`, `>`, `"`, and `'`.
#[inline]
pub fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    // SAFETY: fmt::Write for String is infallible; writing to a String never returns Err.
    write_xml_escaped(&mut out, s)
        .unwrap_or_else(|_| unreachable!("fmt::Write for String is infallible"));
    out
}

/// Write XML-escaped text into a `fmt::Write` destination.
///
/// Uses byte-level batch scanning: writes whole unescaped chunks at once
/// instead of char-by-char.  All five escape characters (`&`, `<`, `>`, `"`, `'`)
/// are single-byte ASCII, so slicing at their positions never splits UTF-8.
pub fn write_xml_escaped<W: fmt::Write>(w: &mut W, s: &str) -> fmt::Result {
    let bytes = s.as_bytes();
    let mut last = 0;
    for (i, &b) in bytes.iter().enumerate() {
        let esc = match b {
            b'&' => "&amp;",
            b'<' => "&lt;",
            b'>' => "&gt;",
            b'"' => "&quot;",
            b'\'' => "&apos;",
            _ => continue,
        };
        if last < i {
            w.write_str(&s[last..i])?;
        }
        w.write_str(esc)?;
        last = i + 1;
    }
    if last < s.len() {
        w.write_str(&s[last..])?;
    }
    Ok(())
}

/// Escape a single character for use in an XML attribute value.
///
/// Returns a `Cow<'static, str>` to avoid heap allocation for the common
/// XML escape sequences (`&amp;`, `&lt;`, etc.).
pub fn xml_escape_char(c: char) -> Cow<'static, str> {
    match c {
        '&' => Cow::Borrowed("&amp;"),
        '<' => Cow::Borrowed("&lt;"),
        '>' => Cow::Borrowed("&gt;"),
        '"' => Cow::Borrowed("&quot;"),
        '\'' => Cow::Borrowed("&apos;"),
        _ => {
            let mut s = String::with_capacity(c.len_utf8());
            s.push(c);
            Cow::Owned(s)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_escape_empty_string() {
        assert_eq!(xml_escape(""), "");
    }

    #[test]
    fn xml_escape_normal_text() {
        assert_eq!(xml_escape("Hello World"), "Hello World");
    }

    #[test]
    fn xml_escape_ampersand() {
        assert_eq!(xml_escape("a & b"), "a &amp; b");
    }

    #[test]
    fn xml_escape_less_than() {
        assert_eq!(xml_escape("a < b"), "a &lt; b");
    }

    #[test]
    fn xml_escape_greater_than() {
        assert_eq!(xml_escape("a > b"), "a &gt; b");
    }

    #[test]
    fn xml_escape_double_quote() {
        assert_eq!(xml_escape(r#"say "hello""#), "say &quot;hello&quot;");
    }

    #[test]
    fn xml_escape_single_quote() {
        assert_eq!(xml_escape("it's"), "it&apos;s");
    }

    #[test]
    fn xml_escape_all_special_chars_combined() {
        assert_eq!(
            xml_escape(r#"<tag attr="val's" & more>"#),
            "&lt;tag attr=&quot;val&apos;s&quot; &amp; more&gt;"
        );
    }

    #[test]
    fn xml_escape_unicode_emoji() {
        // Unicode/emoji should pass through unchanged
        assert_eq!(xml_escape("Hello 🌍"), "Hello 🌍");
    }

    #[test]
    fn xml_escape_unicode_with_special_chars() {
        assert_eq!(xml_escape("🌍 & 🌎"), "🌍 &amp; 🌎");
    }

    #[test]
    fn xml_escape_char_special_chars() {
        assert_eq!(&*xml_escape_char('&'), "&amp;");
        assert_eq!(&*xml_escape_char('<'), "&lt;");
        assert_eq!(&*xml_escape_char('>'), "&gt;");
        assert_eq!(&*xml_escape_char('"'), "&quot;");
        assert_eq!(&*xml_escape_char('\''), "&apos;");
    }

    #[test]
    fn xml_escape_char_normal_char() {
        assert_eq!(&*xml_escape_char('a'), "a");
        assert_eq!(&*xml_escape_char('Z'), "Z");
    }
}
