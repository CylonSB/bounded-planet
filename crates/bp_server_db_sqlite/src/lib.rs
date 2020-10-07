use sqlx::sqlite::SqlitePool;

pub async fn db_connect() -> Result<(), anyhow::Error>
{
    let pool = SqlitePool::new("sqlite::memory:").await?;

    


    todo!()
}