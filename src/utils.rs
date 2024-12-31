/// Truncates a string slice to the new length
pub fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

/// Tries to truncate a string slice to the new length
pub fn try_truncate(s: &str, max_chars: usize) -> Option<&str> {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => Some(&s[..idx]),
        None => None,
    }
}

/// Creates a new string with leading/trailing spaces
pub fn add_padding(s: &str, len: usize, to_right: bool) -> String {
    if s.len() >= len {
        return s.to_owned();
    }

    if to_right {
        format!("{0:>1$}", s, len)
    } else {
        format!("{0:<1$}", s, len)
    }
}
