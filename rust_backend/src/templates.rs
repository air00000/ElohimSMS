use crate::{error::AppError, models::Template};
use sqlx::PgPool;

pub async fn list_templates(pool: &PgPool) -> Result<Vec<Template>, AppError> {
    let templates = sqlx::query_as::<_, Template>(
        "SELECT * FROM templates WHERE is_active = TRUE ORDER BY country_code"
    )
    .fetch_all(pool)
    .await?;

    Ok(templates)
}

pub async fn get_template(pool: &PgPool, country_code: &str) -> Result<Option<Template>, AppError> {
    let template = sqlx::query_as::<_, Template>(
        "SELECT * FROM templates WHERE country_code = UPPER($1) AND is_active = TRUE"
    )
    .bind(country_code)
    .fetch_optional(pool)
    .await?;

    Ok(template)
}

pub async fn upsert_template(
    pool: &PgPool,
    country_code: &str,
    text: &str,
) -> Result<Template, AppError> {
    let country_code = country_code.trim().to_uppercase();
    if country_code.len() != 2 {
        return Err(AppError::BadRequest(
            "Country code must be 2 characters".to_string(),
        ));
    }

    let text = text.trim();
    if text.is_empty() {
        return Err(AppError::BadRequest("Template text cannot be empty".to_string()));
    }

    let template = sqlx::query_as::<_, Template>(
        r#"
        INSERT INTO templates (country_code, text)
        VALUES ($1, $2)
        ON CONFLICT (country_code) DO UPDATE SET
            text = EXCLUDED.text,
            is_active = TRUE,
            updated_at = NOW()
        RETURNING *
        "#,
    )
    .bind(&country_code)
    .bind(text)
    .fetch_one(pool)
    .await?;

    Ok(template)
}

pub async fn delete_template(pool: &PgPool, country_code: &str) -> Result<(), AppError> {
    let result = sqlx::query(
        "UPDATE templates SET is_active = FALSE, updated_at = NOW() WHERE country_code = UPPER($1)"
    )
    .bind(country_code)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(())
}

pub fn render_template(template: &str, link: &str, phone: &str, country_code: &str) -> String {
    template
        .replace("{link}", link)
        .replace("{phone}", phone)
        .replace("{country}", country_code)
}
