use crate::error::SchedulerError;

pub struct Migrator;

impl Migrator {
    pub async fn run(database_url: &str) -> Result<(), SchedulerError> {
        let pool = sqlx::PgPool::connect(database_url)
            .await
            .map_err(|e| SchedulerError::DatabaseError(e.to_string()))?;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| SchedulerError::MigrationError(e.to_string()))?;

        Ok(())
    }
}
