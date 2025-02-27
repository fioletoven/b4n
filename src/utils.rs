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

/// Adds `s` to the specified `text` with padding spaces.
pub fn add_cell(text: &mut String, s: &str, len: usize, to_right: bool) {
    if len == 0 || s.is_empty() {
        return;
    }

    let padding_len = len.saturating_sub(s.chars().count());
    if to_right && padding_len > 0 {
        (0..padding_len).for_each(|_| text.push(' '));
    }

    text.push_str(truncate(s, len));

    if !to_right && padding_len > 0 {
        (0..padding_len).for_each(|_| text.push(' '));
    }
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
