use sha1::{Digest, Sha1};

#[cfg(test)]
#[path = "./utils.tests.rs"]
mod utils_tests;

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

    let start_idx = s.char_indices().nth(total_chars - max_chars).map_or(0, |(idx, _)| idx);

    &s[start_idx..]
}

/// Returns tail of a given string slice.
pub fn slice_from(s: &str, start: usize) -> &str {
    let start_idx = s.char_indices().nth(start).map_or(s.len(), |(i, _)| i);
    &s[start_idx..]
}

/// Returns head of a given string slice.
pub fn slice_to(s: &str, end: usize) -> &str {
    let end_idx = s.char_indices().nth(end).map_or(s.len(), |(i, _)| i);
    &s[..end_idx]
}

/// Returns a substring of a given string slice.
pub fn substring(s: &str, start: usize, len: usize) -> &str {
    let mut iter = s.char_indices();
    let start_idx = iter.nth(start).map_or(s.len(), |(i, _)| i);
    let end_idx = iter.nth(len - 1).map_or(s.len(), |(i, _)| i);

    &s[start_idx..end_idx]
}

/// Returns a substring of a given String.
pub fn substring_owned(mut s: String, start: usize, len: usize) -> String {
    let mut iter = s.char_indices();
    let start_idx = iter.nth(start).map_or(s.len(), |(i, _)| i);
    let end_idx = iter.nth(len - 1).map_or(s.len(), |(i, _)| i);

    s.truncate(end_idx);
    s.drain(..start_idx);

    s
}

/// Finds the start and end (byte indices) of the word that contains the character at `idx`.
pub fn word_bounds(s: &str, idx: usize) -> Option<(usize, usize)> {
    if idx >= s.len() {
        return None;
    }

    let mut start = 0;
    let mut end = 0;

    for (i, ch) in s.char_indices() {
        let is_word = ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == '.' || ch == '/';
        end = i;

        if i < idx && !is_word {
            start = i;
        } else if i >= idx && !is_word {
            end = i.saturating_sub(1);
            break;
        }
    }

    if start > 0 {
        start = std::cmp::min(start + 1, end);
    }

    if start == end || end < idx { None } else { Some((start, end)) }
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
