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

/// Определяет двухбуквенный код страны по международному префиксу.
/// Покрывает наиболее распространённые страны; если код не распознан,
/// возвращает ошибку.
pub fn detect_country_code(phone: &str) -> Result<String, AppError> {
    let normalized = normalize_phone(phone)?;
    let digits: String = normalized.chars().filter(|c| c.is_ascii_digit()).collect();

    // Список префиксов отсортирован по убыванию длины, чтобы сначала
    // проверялись более длинные (трёхзначные) коды.
    let prefixes: [(&str, &str); 41] = [
        ("994", "AZ"),
        ("375", "BY"),
        ("374", "AM"),
        ("373", "MD"),
        ("372", "EE"),
        ("371", "LV"),
        ("370", "LT"),
        ("380", "UA"),
        ("992", "TJ"),
        ("995", "GE"),
        ("996", "KG"),
        ("998", "UZ"),
        ("972", "IL"),
        ("971", "AE"),
        ("966", "SA"),
        ("49", "DE"),
        ("44", "GB"),
        ("33", "FR"),
        ("39", "IT"),
        ("34", "ES"),
        ("41", "CH"),
        ("43", "AT"),
        ("31", "NL"),
        ("32", "BE"),
        ("45", "DK"),
        ("46", "SE"),
        ("47", "NO"),
        ("48", "PL"),
        ("90", "TR"),
        ("20", "EG"),
        ("27", "ZA"),
        ("91", "IN"),
        ("92", "PK"),
        ("86", "CN"),
        ("81", "JP"),
        ("82", "KR"),
        ("65", "SG"),
        ("61", "AU"),
        ("64", "NZ"),
        ("7", "RU"),
        ("1", "US"),
    ];

    for (prefix, country) in prefixes {
        if digits.starts_with(prefix) {
            return Ok(country.to_string());
        }
    }

    Err(AppError::BadRequest(format!(
        "Could not detect country code for phone {phone}"
    )))
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
    fn test_detect_country() {
        assert_eq!(detect_country_code("+19005551234").unwrap(), "US");
        assert_eq!(detect_country_code("+447123456789").unwrap(), "GB");
        assert_eq!(detect_country_code("+4915112345678").unwrap(), "DE");
        assert_eq!(detect_country_code("+79991234567").unwrap(), "RU");
    }
}
