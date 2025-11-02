use std::net::IpAddr;

/// Validator kind that can be used for the filter input.
pub enum ValidatorKind {
    None,
    Number(usize, usize),
    IpAddr,
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
            ValidatorKind::IpAddr => self.validate_ip_address(input),
            ValidatorKind::None => Ok(()),
        }
    }

    fn validate_number(&mut self, input: &str, min: usize, max: usize) -> Result<(), usize> {
        if self.last_validated == input {
            if let Some(index) = self.last_error {
                return Err(index);
            }

            return Ok(());
        }

        input.clone_into(&mut self.last_validated);

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

        if let Ok(x) = input.parse::<usize>()
            && x >= min
            && x <= max
        {
            self.last_error = None;
            return Ok(());
        }

        self.last_error = Some(0);
        Err(0)
    }

    fn validate_ip_address(&mut self, input: &str) -> Result<(), usize> {
        if self.last_validated == input {
            if let Some(index) = self.last_error {
                return Err(index);
            }

            return Ok(());
        }

        input.clone_into(&mut self.last_validated);

        if input.is_empty() {
            self.last_error = None;
            return Ok(());
        }

        if input.parse::<IpAddr>().is_err() {
            self.last_error = Some(0);
            Err(0)
        } else {
            self.last_error = None;
            Ok(())
        }
    }
}
