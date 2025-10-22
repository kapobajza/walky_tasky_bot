use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{storage::base_storage::Storage, task::default::Task};

pub struct InMemoryStorage {
    tasks: RwLock<HashMap<Uuid, Task>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        InMemoryStorage {
            tasks: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Storage for InMemoryStorage {
    async fn save_task(&self, task: Task) -> Result<Uuid, crate::error::SchedulerError> {
        let mut tasks = self.tasks.write().await;
        tasks.insert(task.id, task.clone());
        Ok(task.id)
    }

    async fn get_task(&self, id: uuid::Uuid) -> Result<Option<Task>, crate::error::SchedulerError> {
        let tasks = self.tasks.read().await;
        let task = tasks.get(&id).cloned();
        Ok(task)
    }

    async fn get_all_tasks(&self) -> Result<Vec<Task>, crate::error::SchedulerError> {
        let tasks = self.tasks.read().await;
        Ok(tasks.values().cloned().collect())
    }

    async fn delete_task(&self, id: uuid::Uuid) -> Result<(), crate::error::SchedulerError> {
        let mut tasks = self.tasks.write().await;
        tasks.remove(&id);
        Ok(())
    }

    async fn get_ready_tasks(&self) -> Result<Vec<Task>, crate::error::SchedulerError> {
        let tasks = self.tasks.read().await;
        let now = chrono::Utc::now();
        let ready_tasks: Vec<Task> = tasks
            .values()
            .filter(|task| task.enabled && task.next_run <= now)
            .cloned()
            .collect();
        Ok(ready_tasks)
    }
}
