use regex::Regex;
use std::{collections::HashSet, fmt::Debug, sync::OnceLock};

static ANCHOR_REGEX: OnceLock<Regex> = OnceLock::new();

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
            } else if ampersand_used >= 0 && ampersand_used == (i as isize - 1) && c == '#' {
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

/// Converts full-width digits (０-９) to ASCII digits (0-9)
fn normalize_digits(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '０' => '0',
            '１' => '1',
            '２' => '2',
            '３' => '3',
            '４' => '4',
            '５' => '5',
            '６' => '6',
            '７' => '7',
            '８' => '8',
            '９' => '9',
            _ => c,
        })
        .collect()
}

/// Counts the total number of unique response references (anchors) in the text. (max 20)
/// Anchors start with one or more `>` or `＞` followed by numbers, ranges, and comma-separated lists.
/// Examples:
/// - `>>1` references 1 response
/// - `>>1,2,3` references 3 responses
/// - `>>1-3` references 3 responses (1, 2, 3)
/// - `>>1-2,4,6-8` references 5 responses (1, 2, 4, 6, 7, 8)
pub fn count_anchors(text: &str) -> usize {
    const MAX_ANCHORS_TO_COUNT: usize = 20;

    let re = ANCHOR_REGEX.get_or_init(|| Regex::new(r"[>＞]+([0-9０-９,\-]+)").unwrap());

    let mut all_refs = HashSet::new();

    for cap in re.captures_iter(text) {
        if let Some(number_part) = cap.get(1) {
            // Normalize full-width digits to ASCII
            let normalized = normalize_digits(number_part.as_str());

            // Split by comma to get individual items or ranges
            for item in normalized.split(',') {
                let item = item.trim();
                if item.is_empty() {
                    continue;
                }

                // Check if it's a range (e.g., "1-3")
                if let Some(dash_pos) = item.find('-') {
                    let (start_str, end_str) = item.split_at(dash_pos);
                    let end_str = &end_str[1..]; // Skip the dash

                    if let (Ok(start), Ok(end)) = (start_str.parse::<u32>(), end_str.parse::<u32>())
                    {
                        // Add all numbers in the range, but stop if we reach the limit
                        for n in start..=end {
                            all_refs.insert(n);
                            if all_refs.len() >= MAX_ANCHORS_TO_COUNT {
                                return MAX_ANCHORS_TO_COUNT;
                            }
                        }
                    }
                } else {
                    // Single number
                    if let Ok(n) = item.parse::<u32>() {
                        all_refs.insert(n);
                        if all_refs.len() >= MAX_ANCHORS_TO_COUNT {
                            return MAX_ANCHORS_TO_COUNT;
                        }
                    }
                }
            }
        }
    }

    all_refs.len()
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

    #[test]
    fn test_sanitize_base() {
        assert_eq!(sanitize_base("normal text", false), "normal text");
        assert_eq!(sanitize_base("<script>", false), "&lt;script&gt;");
        assert_eq!(sanitize_base("'\"", false), "&#39;&quot;");
        assert_eq!(sanitize_base("line1\nline2", true), "line1<br>line2");
        assert_eq!(sanitize_base("line1\nline2", false), "line1line2");
        assert_eq!(sanitize_base("test\r\nend", false), "testend");
    }

    #[test]
    fn test_sanitize_num_refs() {
        assert_eq!(sanitize_num_refs("normal text"), "normal text");
        assert_eq!(sanitize_num_refs("test&#xa;end"), "testend");
        assert_eq!(sanitize_num_refs("test&#10;end"), "testend");
        assert_eq!(sanitize_num_refs("test&#X0A;end"), "testend");
        assert_eq!(sanitize_num_refs("keep&#41;this"), "keep)this"); // &#41; is ASCII ')'
        assert_eq!(sanitize_num_refs("keep&#12354;this"), "keep&#12354;this"); // &#12354; is non-ASCII

        // Edge cases: standalone # should not cause panic
        assert_eq!(sanitize_num_refs("#"), "#");
        assert_eq!(sanitize_num_refs("##"), "##");
        assert_eq!(sanitize_num_refs("test#"), "test#");
        assert_eq!(sanitize_num_refs("#test"), "#test");
        assert_eq!(sanitize_num_refs("a#b#c"), "a#b#c");
    }

    #[test]
    fn test_count_anchors() {
        // Single anchor
        assert_eq!(count_anchors(">>1"), 1);
        assert_eq!(count_anchors("＞＞１"), 1);

        // Multiple anchors in comma-separated list
        assert_eq!(count_anchors(">>1,2,3"), 3);
        assert_eq!(count_anchors(">>1,2,3,4"), 4);

        // Range
        assert_eq!(count_anchors(">>1-3"), 3); // 1, 2, 3
        assert_eq!(count_anchors(">>1-5"), 5); // 1, 2, 3, 4, 5

        // Combined: ranges and individual numbers
        assert_eq!(count_anchors(">>1-2,3,4,6-7,8"), 7); // 1, 2, 3, 4, 6, 7, 8
        assert_eq!(count_anchors(">>1-2,4"), 3); // 1, 2, 4

        // Full-width numbers with regular comma
        assert_eq!(count_anchors("＞＞１,２,３"), 3); // １, ２, ３
        assert_eq!(count_anchors("＞＞１-３"), 3); // 1-3

        // Mixed full-width and half-width
        assert_eq!(count_anchors(">>１,2,３"), 3);

        // Multiple anchors in text
        assert_eq!(count_anchors("text >>1 more text >>2,3"), 3); // 1, 2, 3

        // Duplicate references (should count unique only)
        assert_eq!(count_anchors(">>1,1,2,2,3"), 3); // 1, 2, 3 (unique)
        assert_eq!(count_anchors(">>1-3,2-4"), 4); // 1, 2, 3, 4 (unique)

        // No anchors
        assert_eq!(count_anchors("normal text"), 0);

        // Different arrow counts
        assert_eq!(count_anchors(">1"), 1);
        assert_eq!(count_anchors(">>>1"), 1);
        assert_eq!(count_anchors(">>>>1,2,3"), 3);

        // Real-world example
        assert_eq!(count_anchors(">>1\ntest\n>>2-5,7"), 6); // 1, 2, 3, 4, 5, 7

        // Protection against resource exhaustion
        assert_eq!(count_anchors(">>1-1000000"), 20); // Stops at 20
        assert_eq!(count_anchors(">>1-100"), 20); // Also stops at 20
        assert_eq!(
            count_anchors(">>1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22"),
            20
        ); // Stops at 20
    }

    #[test]
    fn test_normalize_digits() {
        assert_eq!(normalize_digits("１２３"), "123");
        assert_eq!(normalize_digits("０"), "0");
        assert_eq!(normalize_digits("９"), "9");
        assert_eq!(
            normalize_digits("mixed １２３ and 456"),
            "mixed 123 and 456"
        );
    }
}
