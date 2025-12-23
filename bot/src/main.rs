use scheduler::{
    db::migrator::Migrator, storage::database_storage::DatabaseStorage,
    task::task_scheduler::TaskScheduler,
};
use std::sync::Arc;
use teloxide::Bot;

use crate::engine::chat_engine::ChatEngine;

mod engine;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let database_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        std::env::var("DB_USER").unwrap_or("postgres".to_string()),
        std::env::var("DB_PASSWORD").unwrap_or("postgres".to_string()),
        std::env::var("DB_HOST").unwrap_or("localhost".to_string()),
        std::env::var("DB_PORT").unwrap_or("5432".to_string()),
        std::env::var("DB_NAME").unwrap_or("wt_db".to_string())
    );

    let db_storage = DatabaseStorage::new(&database_url).await?;

    Migrator::run(&database_url).await?;

    let scheduler = TaskScheduler::new(Arc::new(db_storage));
    scheduler.start().await?;

    let chat_engine = ChatEngine::new(Bot::from_env());
    chat_engine.run().await?;

    Ok(())
}
