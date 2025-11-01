use ulid::Ulid;

/// Generate a new ULID (time-ordered, sortable, unique ID)
pub fn generate_uid() -> String {
    Ulid::new().to_string()
}

/// Validate that a string is a valid ULID
pub fn is_valid_uid(s: &str) -> bool {
    Ulid::from_string(s).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_uid() {
        let uid = generate_uid();
        assert!(!uid.is_empty());
        assert!(is_valid_uid(&uid));
    }

    #[test]
    fn test_is_valid_uid() {
        assert!(is_valid_uid("01ARZ3NDEKTSV4RRFFQ69G5FAV"));
        assert!(!is_valid_uid("invalid"));
        assert!(!is_valid_uid(""));
    }
}

