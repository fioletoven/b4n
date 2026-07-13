use http::uri::Authority;
use std::{net::IpAddr, str::FromStr};

#[cfg(test)]
#[path = "./validator.tests.rs"]
mod validator_tests;

/// Validator kind that can be used for the filter input.
pub enum ValidatorKind {
    None,
    Number(usize, usize),
    StringExcept(Vec<String>),
    StringOneOf(Vec<String>),
    ShellCommand,
    DockerImage,
    IpAddr,
    DnsLabel,
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

        match &self.kind {
            ValidatorKind::Number(min, max) => self.validate_number(input, *min, *max),
            ValidatorKind::StringExcept(except) => validate_sting_except(input, except),
            ValidatorKind::StringOneOf(one_of) => validate_sting_one_of(input, one_of),
            ValidatorKind::ShellCommand => validate_shell_command(input),
            ValidatorKind::DockerImage => self.validate_docker_image(input),
            ValidatorKind::IpAddr => self.validate_ip_address(input),
            ValidatorKind::DnsLabel => self.validate_dns_label(input),
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

    /// Validates a string according to RFC 1123 DNS label rules.
    fn validate_dns_label(&mut self, input: &str) -> Result<(), usize> {
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

        // Each character must be lowercase alphanumeric or '-'.
        for (i, ch) in input.chars().enumerate() {
            if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '-' {
                self.last_error = Some(i);
                return Err(i);
            }
        }

        // Must start with a lowercase alphanumeric character.
        if let Some(first) = input.chars().next()
            && !first.is_ascii_lowercase()
            && !first.is_ascii_digit()
        {
            self.last_error = Some(0);
            return Err(0);
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

    /// Validates a docker container image name.\
    /// Format: `[registry/][namespace/]name[:tag][@digest]`
    fn validate_docker_image(&mut self, input: &str) -> Result<(), usize> {
        input.clone_into(&mut self.last_validated);

        let (image_and_tag, digest) = input.split_once('@').map_or((input, None), |(img, dgst)| (img, Some(dgst)));
        let result = (|| {
            validate_image_and_tag(image_and_tag)?;
            if let Some(digest) = digest {
                validate_digest(digest, image_and_tag.len() + 1)?; // +1 for '@'
            }

            Ok(())
        })();

        self.last_error = result.err();
        result
    }
}

fn validate_sting_except(input: &str, except: &[String]) -> Result<(), usize> {
    if except.contains(&input.to_ascii_lowercase()) {
        Err(0)
    } else {
        Ok(())
    }
}

fn validate_sting_one_of(input: &str, one_of: &[String]) -> Result<(), usize> {
    if one_of.contains(&input.to_ascii_lowercase()) {
        Ok(())
    } else {
        Err(0)
    }
}

/// Validates shell command using `shlex` crate.
fn validate_shell_command(input: &str) -> Result<(), usize> {
    if shlex::split(input).is_some() { Ok(()) } else { Err(0) }
}

/// Format: `[registry/][namespace/]name[:tag]`.
fn validate_image_and_tag(image: &str) -> Result<(), usize> {
    let after_slash = image.rfind('/').map_or(image, |i| &image[i + 1..]);
    let (name, tag) = after_slash.rsplit_once(':').map_or((image, None), |(_, tag)| {
        let name_end = image.len() - tag.len() - 1; // -1 for ':'
        (&image[..name_end], Some(tag))
    });

    validate_image_name(name)?;
    if let Some(tag) = tag {
        validate_image_tag(tag, name.len() + 1)?; // +1 for ':'
    }

    Ok(())
}

/// Format: `[registry/][namespace/]name`.
fn validate_image_name(name: &str) -> Result<(), usize> {
    let mut offset = 0;
    let has_multiple_segments = name.contains('/');

    for (i, segment) in name.split('/').enumerate() {
        if segment.is_empty() {
            return Err(offset);
        }

        if i == 0 && has_multiple_segments && (segment.contains('.') || segment.contains(':')) {
            Authority::from_str(segment).map_err(|_| offset)?;
        } else {
            validate_segment(segment, offset)?;
        }

        offset += segment.len() + 1;
    }

    Ok(())
}

fn validate_image_tag(tag: &str, offset: usize) -> Result<(), usize> {
    if tag.is_empty() {
        return Err(offset);
    }

    if tag.len() > 128 {
        return Err(offset + 128);
    }

    validate_segment(tag, offset)
}

/// Format: `algorithm:hex`.
fn validate_digest(digest: &str, offset: usize) -> Result<(), usize> {
    let Some((algorithm, hex)) = digest.split_once(':') else {
        return Err(offset);
    };

    // Algorithm: [a-z0-9]+, non-empty.
    if algorithm.is_empty() {
        return Err(offset);
    }
    for (i, ch) in algorithm.chars().enumerate() {
        if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() {
            return Err(offset + i);
        }
    }

    // Hex: [a-fA-F0-9]+, non-empty.
    let hex_offset = algorithm.len() + 1; // +1 for ':'
    if hex.is_empty() {
        return Err(offset + hex_offset);
    }
    for (i, ch) in hex.chars().enumerate() {
        if !ch.is_ascii_hexdigit() {
            return Err(offset + hex_offset + i);
        }
    }

    Ok(())
}

fn validate_segment(segment: &str, offset: usize) -> Result<(), usize> {
    if segment.len() > 255 {
        return Err(offset + 255);
    }

    for (i, ch) in segment.chars().enumerate() {
        if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '.' && ch != '_' && ch != '-' {
            return Err(offset + i);
        }
    }

    Ok(())
}
