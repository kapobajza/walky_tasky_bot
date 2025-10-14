use crate::{error::SchedulerError, task::default::Task};
use async_trait::async_trait;

#[async_trait]
pub trait Storage: Send + Sync {
    async fn save_task(&self, task: Task) -> Result<(), SchedulerError>;
    async fn get_task(&self, id: uuid::Uuid) -> Result<Option<Task>, SchedulerError>;
    async fn get_all_tasks(&self) -> Result<Vec<Task>, SchedulerError>;
    async fn delete_task(&self, id: uuid::Uuid) -> Result<(), SchedulerError>;
    async fn get_ready_tasks(&self) -> Result<Vec<Task>, SchedulerError>;
}
