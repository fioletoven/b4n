use ratatui::style::Style;

pub type StyledLine = Vec<(Style, String)>;

pub trait StyledLineExt {
    /// Inserts a character into this `StyledLine` at byte position `idx`.
    fn sl_insert(&mut self, idx: usize, ch: char);

    /// Appends a given string slice to the end of this `StyledLine`.
    fn sl_push_str(&mut self, string: &str);

    /// Appends a character to the back of a `StyledLine`.
    fn sl_push(&mut self, ch: char);

    /// Removes a [`char`] from this `StyledLine` at byte position `idx`.
    fn sl_remove(&mut self, idx: usize);

    /// Splits [`StyledLine`] at byte position `idx` and returns the second part.
    fn get_second(&self, idx: usize) -> StyledLine;

    /// Shortens this `StyledLine` to the specified length.
    fn sl_truncate(&mut self, new_len: usize);
}

impl StyledLineExt for StyledLine {
    fn sl_insert(&mut self, idx: usize, ch: char) {
        let mut current = 0;
        for part in self {
            if current + part.1.len() >= idx {
                part.1.insert(idx - current, ch);
                return;
            }

            current += part.1.len();
        }
    }

    fn sl_push_str(&mut self, string: &str) {
        if let Some(part) = self.last_mut() {
            part.1.push_str(string);
        } else {
            self.push((Style::default(), string.to_owned()));
        }
    }

    fn sl_push(&mut self, ch: char) {
        if let Some(part) = self.last_mut() {
            part.1.push(ch);
        } else {
            self.push((Style::default(), ch.to_string()));
        }
    }

    fn sl_remove(&mut self, idx: usize) {
        let mut current = 0;
        for part in self {
            if current + part.1.len() > idx {
                part.1.remove(idx - current);
                return;
            }

            current += part.1.len();
        }
    }

    fn get_second(&self, idx: usize) -> StyledLine {
        let mut result = Vec::new();
        let mut current = 0;
        let mut is_found = false;
        for part in self {
            if is_found {
                result.push((part.0, part.1.clone()));
            } else if current + part.1.len() > idx {
                result.push((part.0, part.1[idx - current..].to_string()));
                is_found = true;
            }

            current += part.1.len();
        }

        result
    }

    fn sl_truncate(&mut self, new_len: usize) {
        let mut current = 0;
        for (i, part) in self.iter_mut().enumerate() {
            if current + part.1.len() > new_len {
                part.1.truncate(new_len - current);
                if i + 1 < self.len() {
                    self.truncate(i + 1);
                }

                break;
            }

            current += part.1.len();
        }
    }
}
