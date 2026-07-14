use super::*;

#[test]
fn test_valid_images() {
    let mut validator = InputValidator::new(ValidatorKind::DockerImage);

    assert!(validator.validate("ubuntu").is_ok());
    assert!(validator.validate("ubuntu:22.04").is_ok());
    assert!(validator.validate("ubuntu:latest").is_ok());
    assert!(validator.validate("myrepo/myimage").is_ok());
    assert!(validator.validate("myrepo/myimage:1.0.0").is_ok());
    assert!(validator.validate("registry.example.com:5000/myimage").is_ok());
    assert!(validator.validate("registry.example.com:5000/myimage:latest").is_ok());
    assert!(validator.validate("registry.example.com/org/myimage:1.0").is_ok());
    assert!(validator.validate("ubuntu:sha256-abc123").is_ok());
    assert!(validator.validate("ubuntu:123@sha256:abc123").is_ok());
    assert!(validator.validate("example.com:5000/ubuntu:123@sha256:abc123").is_ok());
    assert!(validator.validate("myimage:123").is_ok());
}

#[test]
fn test_invalid_images() {
    let mut validator = InputValidator::new(ValidatorKind::DockerImage);

    assert!(validator.validate("ubuntu:").is_err());
    assert!(validator.validate(":latest").is_err());
    assert!(validator.validate("my image").is_err());
    assert!(validator.validate("my@image").is_err());
    assert!(validator.validate("myimage:latest!").is_err());
    assert!(validator.validate("myimage:my tag").is_err());
    assert!(validator.validate(&format!("myimage:{}", "a".repeat(129))).is_err());
    assert!(validator.validate(&"a".repeat(256)).is_err());
    assert!(validator.validate("my::image").is_err());
    assert!(validator.validate("MyImage").is_err());
}
