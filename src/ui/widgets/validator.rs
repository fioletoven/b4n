/// Validator kind that can be used for the filter input.
pub enum ValidatorKind {
    None,
    Number(usize, usize),
}

pub struct InputValidator {
    kind: ValidatorKind,
    last_validated: String,
    last_error: Option<usize>,
}

impl InputValidator {
    /// Creates new [`InputValidator`] instance.
    pub fn new(kind: ValidatorKind) -> Self {
        Self {
            kind,
            last_validated: String::default(),
            last_error: None,
        }
    }

    /// Validates specified input.
    pub fn validate(&mut self, input: &str) -> Result<(), usize> {
        match self.kind {
            ValidatorKind::Number(min, max) => self.validate_number(input, min, max),
            _ => Ok(()),
        }
    }

    fn validate_number(&mut self, input: &str, min: usize, max: usize) -> Result<(), usize> {
        if self.last_validated == input {
            if let Some(index) = self.last_error {
                return Err(index);
            } else {
                return Ok(());
            }
        }

        self.last_validated = input.to_owned();

        if input.is_empty() {
            self.last_error = None;
            return Ok(());
        }

        for (i, ch) in input.chars().enumerate() {
            if !ch.is_numeric() {
                self.last_error = Some(i);
                return Err(i);
            }
        }

        if let Ok(x) = input.parse::<usize>() {
            if x >= min && x <= max {
                self.last_error = None;
                return Ok(());
            }
        }

        self.last_error = Some(0);
        Err(0)
    }
}
