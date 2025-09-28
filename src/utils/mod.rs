pub mod logical_expressions;

use sha1::{Digest, Sha1};

/// Truncates a string slice to the new length.
pub fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

/// Tries to truncate a string slice to the new length.
pub fn try_truncate(s: &str, max_chars: usize) -> Option<&str> {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => Some(&s[..idx]),
        None => None,
    }
}

/// Truncates a string slice from the left to the new length.
pub fn truncate_left(s: &str, max_chars: usize) -> &str {
    let total_chars = s.chars().count();
    if max_chars >= total_chars {
        return s;
    }

    let start_idex = s.char_indices().nth(total_chars - max_chars).map(|(idx, _)| idx).unwrap_or(0);

    &s[start_idex..]
}

/// Adds padding to the string slice.
pub fn add_padding(s: &str, width: usize) -> String {
    let name_width = s.chars().count();

    let mut text = String::with_capacity(width);
    text.push_str(truncate(s, width));

    let padding_len = width.saturating_sub(name_width);
    (0..padding_len).for_each(|_| text.push(' '));

    text
}

/// Calculates SHA1 for specified string and sets the length to `len`.
pub fn calculate_hash(t: &str, len: usize) -> String {
    let mut hasher = Sha1::new();
    hasher.update(t);
    let mut hash = format!("{:x}", hasher.finalize());
    if len > 0 {
        hash.truncate(len);
    }

    hash
}
