use sqlx::{migrate::MigrateDatabase, Pool, Postgres};

pub async fn init_db(database_url: &str) -> anyhow::Result<Pool<Postgres>> {
    if !Postgres::database_exists(database_url).await.unwrap_or(false) {
        Postgres::create_database(database_url).await?;
    }

    let pool = Pool::<Postgres>::connect(database_url).await?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    Ok(pool)
}
