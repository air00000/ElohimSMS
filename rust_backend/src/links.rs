use crate::{error::AppError, models::Campaign};
use rand::Rng;
use sqlx::PgPool;
use uuid::Uuid;

const ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
const SHORT_CODE_LEN: usize = 8;

fn generate_short_code() -> String {
    let mut rng = rand::thread_rng();
    (0..SHORT_CODE_LEN)
        .map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char)
        .collect()
}

pub async fn create_link(
    pool: &PgPool,
    target_url: &str,
    phone: &str,
    country_code: &str,
    message: &str,
) -> Result<Campaign, AppError> {
    for _ in 0..10 {
        let short_code = generate_short_code();

        let result = sqlx::query_as::<_, Campaign>(
            r#"
            INSERT INTO campaigns (short_code, target_url, phone, country_code, message)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(&short_code)
        .bind(target_url)
        .bind(phone)
        .bind(country_code)
        .bind(message)
        .fetch_one(pool)
        .await;

        match result {
            Ok(campaign) => return Ok(campaign),
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => continue,
            Err(e) => return Err(e.into()),
        }
    }

    Err(AppError::Internal(
        "Failed to generate unique short code".to_string(),
    ))
}

pub async fn get_campaign_by_short_code(
    pool: &PgPool,
    short_code: &str,
) -> Result<Option<Campaign>, AppError> {
    let campaign = sqlx::query_as::<_, Campaign>(
        "SELECT * FROM campaigns WHERE short_code = $1"
    )
    .bind(short_code)
    .fetch_optional(pool)
    .await?;

    Ok(campaign)
}

pub async fn increment_click_count(pool: &PgPool, campaign_id: Uuid) -> Result<(), AppError> {
    sqlx::query("UPDATE campaigns SET click_count = click_count + 1 WHERE id = $1")
        .bind(campaign_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn mark_verified(pool: &PgPool, campaign_id: Uuid) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE campaigns SET verified_count = verified_count + 1 WHERE id = $1"
    )
    .bind(campaign_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub fn build_short_link(captcha_site_url: &str, short_code: &str) -> String {
    format!("{}/l/{}", captcha_site_url, short_code)
}
