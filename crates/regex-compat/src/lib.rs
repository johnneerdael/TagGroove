use std::{
    ffi::{c_char, c_void, CStr},
    ptr::NonNull,
};

extern "C" {
    fn ot_regex_new(text: *const u16, count: usize, sensitive: bool) -> *mut c_void;
    fn ot_regex_free(expression: *mut c_void);
    fn ot_regex_error() -> *const c_char;
    fn ot_regex_groups(expression: *const c_void) -> usize;
    fn ot_regex_captures(
        expression: *const c_void,
        text: *const u16,
        count: usize,
        positions: *mut usize,
        lengths: *mut usize,
        capacity: usize,
    ) -> i32;
    fn ot_regex_replace(
        expression: *const c_void,
        text: *const u16,
        count: usize,
        replacement: *const u16,
        replacement_count: usize,
        output_count: *mut usize,
    ) -> *mut u16;
    fn ot_regex_output_free(text: *mut u16);
}

fn error() -> String {
    // The native error buffer is thread-local and read immediately after failure.
    unsafe {
        CStr::from_ptr(ot_regex_error())
            .to_string_lossy()
            .into_owned()
    }
}

/// A compiled Boost Perl expression. The handle stays on its creating thread.
pub struct Regex {
    handle: NonNull<c_void>,
}
impl std::fmt::Debug for Regex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoostRegex").finish_non_exhaustive()
    }
}
impl Regex {
    pub fn new(pattern: &str, case_sensitive: bool) -> Result<Self, String> {
        let pattern: Vec<u16> = ascii_digit_classes(pattern)?.encode_utf16().collect();
        // Native code reads exactly count UTF-16 units and catches C++ exceptions.
        NonNull::new(unsafe { ot_regex_new(pattern.as_ptr(), pattern.len(), case_sensitive) })
            .map(|handle| Self { handle })
            .ok_or_else(error)
    }
    pub fn is_match(&self, input: &str) -> Result<bool, String> {
        let input: Vec<u16> = input.encode_utf16().collect();
        match unsafe {
            ot_regex_captures(
                self.handle.as_ptr(),
                input.as_ptr(),
                input.len(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
            )
        } {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(error()),
        }
    }
    pub fn replace(&self, input: &str, replacement: &str) -> Result<String, String> {
        let input: Vec<u16> = input.encode_utf16().collect();
        let replacement: Vec<u16> = replacement.encode_utf16().collect();
        let mut count = 0;
        let result = NonNull::new(unsafe {
            ot_regex_replace(
                self.handle.as_ptr(),
                input.as_ptr(),
                input.len(),
                replacement.as_ptr(),
                replacement.len(),
                &mut count,
            )
        })
        .ok_or_else(error)?;
        // Output is allocated by the bridge; copy it before releasing with its allocator.
        let output =
            unsafe { String::from_utf16_lossy(std::slice::from_raw_parts(result.as_ptr(), count)) };
        unsafe {
            ot_regex_output_free(result.as_ptr());
        }
        Ok(output)
    }
    pub fn captures(&self, input: &str) -> Result<Option<Vec<Option<String>>>, String> {
        let input: Vec<u16> = input.encode_utf16().collect();
        let count = unsafe { ot_regex_groups(self.handle.as_ptr()) };
        let mut positions = vec![usize::MAX; count];
        let mut lengths = vec![0; count];
        match unsafe {
            ot_regex_captures(
                self.handle.as_ptr(),
                input.as_ptr(),
                input.len(),
                positions.as_mut_ptr(),
                lengths.as_mut_ptr(),
                count,
            )
        } {
            0 => Ok(None),
            1 => Ok(Some(
                positions
                    .into_iter()
                    .zip(lengths)
                    .map(|(position, length)| {
                        if position == usize::MAX {
                            None
                        } else {
                            Some(String::from_utf16_lossy(
                                &input[position..position + length],
                            ))
                        }
                    })
                    .collect(),
            )),
            _ => Err(error()),
        }
    }
}
impl Drop for Regex {
    fn drop(&mut self) {
        unsafe {
            ot_regex_free(self.handle.as_ptr());
        }
    }
}

// Mp3tag explicitly documents \d as 0-9. Boost wide traits otherwise inherit
// the host locale's broader digit category. Retain all other Perl syntax.
fn ascii_digit_classes(pattern: &str) -> Result<String, String> {
    let chars: Vec<char> = pattern.chars().collect();
    let mut output = String::new();
    let mut i = 0;
    let mut quoted = false;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            let next = chars[i + 1];
            if next == 'Q' {
                quoted = true;
            }
            if next == 'E' {
                quoted = false;
            }
            if !quoted && matches!(next, 'd' | 'D') {
                output.push_str(if next == 'd' { "[0-9]" } else { "[^0-9]" });
            } else {
                output.push('\\');
                output.push(next);
            }
            i += 2;
            continue;
        }
        if chars[i] != '[' || quoted {
            output.push(chars[i]);
            i += 1;
            continue;
        }
        let start = i;
        i += 1;
        let negative = chars.get(i) == Some(&'^');
        if negative {
            i += 1;
        }
        let mut body = String::new();
        let mut non_digit = false;
        if chars.get(i) == Some(&']') {
            body.push(']');
            i += 1;
        }
        while i < chars.len() && chars[i] != ']' {
            if chars[i] == '\\' && i + 1 < chars.len() {
                match chars[i + 1] {
                    'd' => body.push_str("0-9"),
                    'D' => non_digit = true,
                    next => {
                        body.push('\\');
                        body.push(next);
                    }
                }
                i += 2;
            } else if chars[i] == '['
                && chars
                    .get(i + 1)
                    .is_some_and(|c| matches!(c, ':' | '.' | '='))
            {
                let delimiter = chars[i + 1];
                body.push('[');
                body.push(delimiter);
                i += 2;
                while i < chars.len() && !(chars[i] == delimiter && chars.get(i + 1) == Some(&']'))
                {
                    body.push(chars[i]);
                    i += 1;
                }
                if i + 1 >= chars.len() {
                    return Err("Unterminated POSIX character class".into());
                }
                body.push(delimiter);
                body.push(']');
                i += 2;
            } else {
                body.push(chars[i]);
                i += 1;
            }
        }
        if i == chars.len() {
            return Err(format!("Unterminated character class at {start}"));
        }
        i += 1;
        if !non_digit {
            output.push('[');
            if negative {
                output.push('^');
            }
            output.push_str(&body);
            output.push(']');
        } else if body.is_empty() {
            output.push_str(if negative { "[0-9]" } else { "[^0-9]" });
        } else if negative {
            output.push_str(&format!("(?:(?!(?:[{body}]|[^0-9]))[\\s\\S])"));
        } else {
            output.push_str(&format!("(?:[{body}]|[^0-9])"));
        }
    }
    Ok(output)
}
pub fn replace(
    input: &str,
    pattern: &str,
    replacement: &str,
    case_sensitive: bool,
) -> Result<String, String> {
    Regex::new(pattern, case_sensitive)?.replace(input, replacement)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documented_character_classes_and_repetition() {
        for (pattern, input) in [
            (r"^\l+\u+$", "abCD"),
            (r"^\L+\U+$", "CDab"),
            (r"^\d{2}\D+$", "12é"),
            (r"^\w+\W\s\S$", "abc! X"),
            (r"^\x{732B}\xE9$", "猫é"),
            (r"^a.*?z$", "abcz"),
            (r"^(foo|bar)?\t\r\n$", "foo\t\r\n"),
            (r"\bcat\b", "a cat."),
        ] {
            assert!(
                Regex::new(pattern, true).unwrap().is_match(input).unwrap(),
                "{pattern}"
            );
        }
        assert!(!Regex::new(r"^\d+$", true)
            .unwrap()
            .is_match("１２")
            .unwrap());
        assert!(Regex::new(r"^[a\D]+$", true)
            .unwrap()
            .is_match("１２a")
            .unwrap());
        assert!(!Regex::new(r"^[^a\D]+$", true)
            .unwrap()
            .is_match("１２")
            .unwrap());
        assert!(Regex::new(r"^[^a\D]+$", true)
            .unwrap()
            .is_match("12")
            .unwrap());
    }

    #[test]
    fn lookaround_pattern_backreferences_and_perl_replacement() {
        assert_eq!(
            replace("abc def", r"(?<=\b)(\w+)(?=\b)", r"\U$1\E", true).unwrap(),
            "ABC DEF"
        );
        assert_eq!(
            replace("abc abc xyz", r"\b(\w+) \1\b", "$1", true).unwrap(),
            "abc xyz"
        );
        assert_eq!(replace("Alpha", "alpha", "$&!", false).unwrap(), "Alpha!");
        assert_eq!(replace("猫", "(猫)", "$1$1", true).unwrap(), "猫猫");
    }

    #[test]
    fn captures_keep_unmatched_optional_groups() {
        assert_eq!(
            Regex::new("(foo)(bar)?", true)
                .unwrap()
                .captures("foo")
                .unwrap(),
            Some(vec![Some("foo".into()), Some("foo".into()), None])
        );
        assert_eq!(
            Regex::new("x", true).unwrap().captures("foo").unwrap(),
            None
        );
    }

    #[test]
    fn malformed_patterns_report_errors_and_empty_matches_terminate() {
        assert!(Regex::new("(", true).is_err());
        assert!(Regex::new("[[:digit:", true).is_err());
        assert_eq!(replace("ab", "", "_", true).unwrap(), "_a_b_");
    }
}
