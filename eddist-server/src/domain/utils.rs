use regex::Regex;
use std::fmt::Debug;

#[derive(Clone)]
pub struct SimpleSecret(String);

impl SimpleSecret {
    pub fn new(secret: &str) -> Self {
        Self(secret.to_string())
    }

    pub fn get(&self) -> &str {
        &self.0
    }
}

impl Debug for SimpleSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[REDACTED]")
    }
}

pub fn sanitize_base(input: &str, is_body: bool) -> String {
    input
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
        .replace('\r', "")
        .replace('\n', if is_body { "<br>" } else { "" })
        .replace("&NewLine;", "")
}

pub fn sanitize_num_refs(input: &str) -> String {
    // Delete all of semicolon closing \n character references
    let re = Regex::new(r"&#([Xx]0*[aA]|0*10);").unwrap();
    let rn_sanitized = re.replace_all(input, "");

    sanitize_ascii_numeric_character_reference(&sanitize_non_semi_closing_num_char_refs(
        &rn_sanitized,
    ))
}

// Delete all of non-semicolon closing numeric character references
fn sanitize_non_semi_closing_num_char_refs(target: &str) -> String {
    let mut sanitized = Vec::new();
    let mut ampersand_used = -1;
    let mut total_removed_len = 0;
    enum NumRefKind {
        Undef, // this state is only cause after reading "&#"
        Hex,
        Dec,
    }
    let mut in_num_ref = None;
    for (i, c) in target.chars().enumerate() {
        if let Some(kind) = &in_num_ref {
            if c == ';' {
                in_num_ref = None;
                sanitized.push(c);
            } else {
                match kind {
                    NumRefKind::Undef => {
                        match c {
                            'x' | 'X' => in_num_ref = Some(NumRefKind::Hex),
                            '0'..='9' => in_num_ref = Some(NumRefKind::Dec),
                            _ => in_num_ref = None,
                        };
                        sanitized.push(c);
                    }
                    NumRefKind::Hex => match c {
                        '0'..='9' | 'a'..='f' | 'A'..='F' => sanitized.push(c),
                        _ => {
                            // invalid non-semicolon closing numeric character references
                            in_num_ref = None;
                            sanitized =
                                sanitized[0..ampersand_used as usize - total_removed_len].to_vec();
                            total_removed_len += i - ampersand_used as usize;
                            sanitized.push(c);
                            if c == '&' {
                                ampersand_used = i as isize;
                            }
                        }
                    },
                    NumRefKind::Dec => match c {
                        '0'..='9' => sanitized.push(c),
                        _ => {
                            // invalid non-semicolon closing numeric character references
                            in_num_ref = None;
                            sanitized =
                                sanitized[0..ampersand_used as usize - total_removed_len].to_vec();
                            total_removed_len += i - ampersand_used as usize;
                            sanitized.push(c);
                            if c == '&' {
                                ampersand_used = i as isize;
                            }
                        }
                    },
                }
            }
        } else {
            sanitized.push(c);
            if c == '&' {
                ampersand_used = i as isize;
            } else if ampersand_used == (i as isize - 1) && c == '#' {
                in_num_ref = Some(NumRefKind::Undef);
            }
        }
    }

    if in_num_ref.is_some() {
        sanitized = sanitized[0..ampersand_used as usize - total_removed_len].to_vec();
    }

    sanitized.into_iter().collect::<String>()
}

// Delete all ascii numeric character reference
pub fn sanitize_ascii_numeric_character_reference(input: &str) -> String {
    let mut sanitized = Vec::new();

    let mut iter = input.chars().peekable();
    while let Some(c) = iter.next() {
        if c == '&' && iter.peek() == Some(&'#') {
            let mut original = String::new();
            original.push(c);
            // consume '#' and record it
            let hash = iter.next().unwrap();
            original.push(hash);

            let mut num_str = String::new();
            let mut is_hex = false;
            if let Some(&next) = iter.peek() {
                if next == 'x' || next == 'X' {
                    is_hex = true;
                    original.push(iter.next().unwrap());
                }
            }
            while let Some(&next) = iter.peek() {
                if (is_hex && next.is_ascii_hexdigit()) || (!is_hex && next.is_ascii_digit()) {
                    num_str.push(next);
                    original.push(iter.next().unwrap());
                } else {
                    break;
                }
            }
            if let Some(&';') = iter.peek() {
                original.push(iter.next().unwrap());
                if !num_str.is_empty() {
                    let parsed = if is_hex {
                        u32::from_str_radix(&num_str, 16)
                    } else {
                        num_str.parse::<u32>()
                    };
                    if let Ok(code_point) = parsed {
                        if let Some(ch) = std::char::from_u32(code_point) {
                            // Replace only if the character is ASCII
                            if ch.is_ascii() {
                                sanitized.push(ch);
                                continue;
                            }
                        }
                    }
                }
            }
            // If conversion did not happen, push back the original text.
            sanitized.extend(original.chars());
        } else {
            sanitized.push(c);
        }
    }

    sanitized.into_iter().collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_ascii_numeric_character_reference() {
        let input = "Hello &amp; &#x41; &#65; World!";
        let expected = "Hello &amp; A A World!";
        let result = sanitize_ascii_numeric_character_reference(input);
        assert_eq!(expected, result);

        let input = "Invalid & #x41; &#65 World!";
        let expected = "Invalid & #x41; &#65 World!";
        let result = sanitize_ascii_numeric_character_reference(input);
        assert_eq!(expected, result);

        let input = "non ascii &#x41; &#65; &#x1F600; &#128512;";
        let expected = "non ascii A A &#x1F600; &#128512;";
        let result = sanitize_ascii_numeric_character_reference(input);
        assert_eq!(expected, result);

        let input = "&#x0000000000041; &#xx000031; &#00000065; &#x1F600; &#128512;";
        let expected = "A &#xx000031; A &#x1F600; &#128512;";
        let result = sanitize_ascii_numeric_character_reference(input);
        assert_eq!(expected, result);
    }
}
