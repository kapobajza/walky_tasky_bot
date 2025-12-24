use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    error::SchedulerError,
    storage::base_storage::Storage,
    task::default::{Task, TaskDb},
};

pub struct DatabaseStorage {
    pub pool: sqlx::PgPool,
}

impl DatabaseStorage {
    pub async fn new(database_url: &str) -> Result<Self, SchedulerError> {
        let pool = sqlx::PgPool::connect(database_url)
            .await
            .map_err(|e| SchedulerError::DatabaseError(e.to_string()))?;
        Ok(DatabaseStorage { pool })
    }
}

#[async_trait]
impl Storage for DatabaseStorage {
    async fn save_task(&self, task: Task) -> Result<Uuid, crate::error::SchedulerError> {
        let db_task = Task::to_db_task(&task)?;

        let task_id = sqlx::query_scalar!(
            "INSERT INTO tasks (id, schedule_type, last_run, next_run, retry_count, max_retries, retry_delay, schedule, enabled)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (id) DO UPDATE SET
                schedule_type = EXCLUDED.schedule_type,
                last_run = EXCLUDED.last_run,
                next_run = EXCLUDED.next_run,
                retry_count = EXCLUDED.retry_count,
                max_retries = EXCLUDED.max_retries,
                retry_delay = EXCLUDED.retry_delay,
                schedule = EXCLUDED.schedule,
                enabled = EXCLUDED.enabled
            RETURNING id",
            db_task.id,
            db_task.schedule_type,
            db_task.last_run,
            db_task.next_run,
            db_task.retry_count,
            db_task.max_retries,
            db_task.retry_delay,
            db_task.schedule,
            db_task.enabled
        ).fetch_one(&self.pool)
            .await
            .map_err(|e| SchedulerError::DatabaseError(e.to_string()))?;

        Ok(task_id)
    }

    async fn get_task(&self, id: uuid::Uuid) -> Result<Option<Task>, crate::error::SchedulerError> {
        let record = sqlx::query_as!(
            TaskDb,
            "SELECT id, schedule_type as \"schedule_type: i16\", last_run, next_run, retry_count, max_retries, retry_delay, schedule, enabled
            FROM tasks WHERE id = $1",
            id
        ).fetch_optional(&self.pool)
            .await
            .map_err(|e| SchedulerError::DatabaseError(e.to_string()))?;

        Ok(record.map(Task::from_db_task).transpose()?)
    }

    async fn get_all_tasks(&self) -> Result<Vec<Task>, crate::error::SchedulerError> {
        let records = sqlx::query_as!(
            TaskDb,
            "SELECT id, schedule_type as \"schedule_type: i16\", last_run, next_run, retry_count, max_retries, retry_delay, schedule, enabled
            FROM tasks"
        ).fetch_all(&self.pool)
            .await
            .map_err(|e| SchedulerError::DatabaseError(e.to_string()))?;

        let tasks: Result<Vec<Task>, SchedulerError> =
            records.into_iter().map(Task::from_db_task).collect();
        Ok(tasks?)
    }

    async fn delete_task(&self, id: uuid::Uuid) -> Result<(), crate::error::SchedulerError> {
        sqlx::query!("DELETE FROM tasks WHERE id = $1", id)
            .execute(&self.pool)
            .await
            .map_err(|e| SchedulerError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    async fn get_ready_tasks(&self) -> Result<Vec<Task>, crate::error::SchedulerError> {
        let records = sqlx::query_as!(
            TaskDb,
            "SELECT id, schedule_type as \"schedule_type: i16\", last_run, next_run, retry_count, max_retries, retry_delay, schedule, enabled
            FROM tasks WHERE next_run <= NOW() AND enabled = TRUE",
        ).fetch_all(&self.pool)
            .await
            .map_err(|e| SchedulerError::DatabaseError(e.to_string()))?;

        let tasks: Result<Vec<Task>, SchedulerError> =
            records.into_iter().map(Task::from_db_task).collect();
        Ok(tasks?)
    }
}
