use once_cell::sync::Lazy;
use regex::Regex;

static PHONE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\+7[0-9]{10}$").unwrap());

static EMAIL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap());

static BIN_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[0-9]{12}$").unwrap());

pub fn validate_phone(phone: &str) -> bool {
    PHONE_REGEX.is_match(phone)
}

pub fn validate_email(email: &str) -> bool {
    EMAIL_REGEX.is_match(email)
}

pub fn validate_bin(bin: &str) -> bool {
    BIN_REGEX.is_match(bin)
}

pub fn sanitize_string(input: &str) -> String {
    input.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_phone() {
        assert!(validate_phone("+77771234567"));
        assert!(!validate_phone("87771234567"));
        assert!(!validate_phone("+7777123456"));
        assert!(!validate_phone("+777712345678"));
    }

    #[test]
    fn test_validate_email() {
        assert!(validate_email("test@example.com"));
        assert!(validate_email("user.name@domain.co.kz"));
        assert!(!validate_email("invalid"));
        assert!(!validate_email("@example.com"));
    }

    #[test]
    fn test_validate_bin() {
        assert!(validate_bin("123456789012"));
        assert!(!validate_bin("12345678901"));
        assert!(!validate_bin("1234567890123"));
    }
}
