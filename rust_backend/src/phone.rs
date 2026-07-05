use crate::error::AppError;

pub fn normalize_phone(phone: &str) -> Result<String, AppError> {
    let trimmed = phone.trim();

    if trimmed.is_empty() {
        return Err(AppError::BadRequest("Phone number is required".to_string()));
    }

    // Убираем пробелы, скобки, дефисы
    let cleaned: String = trimmed
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '(' && *c != ')' && *c != '-' && *c != '.')
        .collect();

    if !cleaned.starts_with('+') {
        return Err(AppError::BadRequest(
            "Phone number must start with '+' and country code".to_string(),
        ));
    }

    let digits_only: String = cleaned.chars().filter(|c| c.is_ascii_digit()).collect();

    if digits_only.len() < 10 || digits_only.len() > 15 {
        return Err(AppError::BadRequest(
            "Phone number must contain between 10 and 15 digits".to_string(),
        ));
    }

    Ok(format!("+{}", digits_only))
}

pub fn detect_country_code(phone: &str) -> Result<String, AppError> {
    let normalized = normalize_phone(phone)?;

    let parsed = phonenumber::parse(None, &normalized).map_err(|e| {
        AppError::BadRequest(format!("Failed to parse phone number: {e}"))
    })?;

    if !phonenumber::is_valid(&parsed) {
        return Err(AppError::BadRequest("Phone number is not valid".to_string()));
    }

    let country_code = parsed
        .country()
        .id()
        .map(|id| id.as_ref().to_string())
        .ok_or_else(|| {
            AppError::BadRequest("Could not detect country from phone number".to_string())
        })?;

    Ok(country_code.to_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_valid_phone() {
        assert_eq!(
            normalize_phone("+7 (999) 123-45-67").unwrap(),
            "+79991234567"
        );
    }

    #[test]
    fn test_normalize_missing_plus() {
        assert!(normalize_phone("79991234567").is_err());
    }

    #[test]
    fn test_normalize_too_short() {
        assert!(normalize_phone("+7999").is_err());
    }

    #[test]
    fn test_detect_country_us() {
        assert_eq!(detect_country_code("+19005551234").unwrap(), "US");
    }

    #[test]
    fn test_detect_country_gb() {
        assert_eq!(detect_country_code("+447123456789").unwrap(), "GB");
    }
}
