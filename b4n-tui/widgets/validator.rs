use std::net::IpAddr;

/// Validator kind that can be used for the filter input.
pub enum ValidatorKind {
    None,
    Number(usize, usize),
    IpAddr,
    Namespace,
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
        if self.last_validated == input {
            return match self.last_error {
                Some(idx) => Err(idx),
                None => Ok(()),
            };
        }

        match self.kind {
            ValidatorKind::Number(min, max) => self.validate_number(input, min, max),
            ValidatorKind::IpAddr => self.validate_ip_address(input),
            ValidatorKind::Namespace => self.validate_namespace(input),
            ValidatorKind::None => Ok(()),
        }
    }

    fn validate_number(&mut self, input: &str, min: usize, max: usize) -> Result<(), usize> {
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

    /// Validates a Kubernetes namespace name according to RFC 1123 DNS label rules.
    fn validate_namespace(&mut self, input: &str) -> Result<(), usize> {
        input.clone_into(&mut self.last_validated);

        if input.is_empty() {
            self.last_error = None;
            return Ok(());
        }

        // Max length is 63 characters.
        if input.len() > 63 {
            self.last_error = Some(63);
            return Err(63);
        }

        // Must start with a lowercase alphanumeric character.
        if let Some(first) = input.chars().next()
            && !first.is_ascii_lowercase()
            && !first.is_ascii_digit()
        {
            self.last_error = Some(0);
            return Err(0);
        }

        // Each character must be lowercase alphanumeric or '-'.
        for (i, ch) in input.chars().enumerate() {
            if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '-' {
                self.last_error = Some(i);
                return Err(i);
            }
        }

        // Must end with a lowercase alphanumeric character.
        if let Some(last) = input.chars().last()
            && !last.is_ascii_lowercase()
            && !last.is_ascii_digit()
        {
            let last_index = input.len() - 1;
            self.last_error = Some(last_index);
            return Err(last_index);
        }

        self.last_error = None;
        Ok(())
    }
}
