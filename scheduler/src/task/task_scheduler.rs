use std::{collections::HashMap, sync::Arc, time::Duration};

use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    error::SchedulerError,
    storage::base_storage::Storage,
    task::{task::Task, task_handler::TaskHandler},
};

pub struct TaskScheduler {
    storage: Arc<dyn Storage>,
    handlers: Arc<RwLock<HashMap<Uuid, Arc<dyn TaskHandler>>>>,
    running: Arc<RwLock<bool>>,
    check_interval: Duration,
}

impl TaskScheduler {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self {
            storage,
            handlers: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
            check_interval: Duration::from_millis(500),
        }
    }

    pub fn with_check_interval(mut self, interval: Duration) -> Self {
        self.check_interval = interval;
        self
    }

    pub async fn add_task(
        &self,
        task: Task,
        handler: Arc<dyn TaskHandler>,
    ) -> Result<Uuid, SchedulerError> {
        self.storage.save_task(task.clone()).await?;

        let mut handlers = self.handlers.write().await;
        handlers.insert(task.id, handler);

        Ok(task.id)
    }

    async fn execute_task_with_retry(
        handler: Arc<dyn TaskHandler>,
        mut task: Task,
        storage: Arc<dyn Storage>,
    ) {
        loop {
            match handler.execute(&task).await {
                Ok(_) => {
                    println!("Task {} executed successfully", task.id);
                    task.reset_retry_count();
                    task.last_run = Some(chrono::Utc::now());

                    if let Err(e) = task.calculate_next_run() {
                        eprintln!("Error calculating next run for task {}: {:?}", task.id, e);
                        return;
                    }

                    if let Err(e) = storage.save_task(task).await {
                        eprintln!("Error updating task {:?}", e);
                    }
                    return;
                }
                Err(e) => {
                    task.retry_count += 1;
                    eprintln!(
                        "Error executing task {}: {:?}. Retry count: {}",
                        task.id, e, task.retry_count
                    );

                    if task.should_retry() {
                        let retry_delay = task.calcluate_retry_delay();
                        tokio::time::sleep(retry_delay).await;
                        continue;
                    } else {
                        eprintln!("Max retries reached for task {}. Giving up.", task.id);
                        task.last_run = Some(chrono::Utc::now());

                        if let Err(e) = task.calculate_next_run() {
                            eprintln!("Error calculating next run for task {}: {:?}", task.id, e);
                        } else {
                            task.reset_retry_count();
                        }

                        if let Err(e) = storage.save_task(task).await {
                            eprintln!("Error updating task {:?}", e);
                        }
                        return;
                    }
                }
            }
        }
    }

    pub async fn start(&self) -> Result<(), SchedulerError> {
        {
            let mut running = self.running.write().await;
            if *running {
                return Err(SchedulerError::AlreadyRunning);
            }
            *running = true;
        }

        let storage = Arc::clone(&self.storage);
        let handlers = Arc::clone(&self.handlers);
        let running = Arc::clone(&self.running);
        let check_interval = self.check_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(check_interval);

            loop {
                interval.tick().await;

                {
                    let should_run = *running.read().await;
                    if !should_run {
                        break;
                    }
                }

                match storage.get_ready_tasks().await {
                    Ok(ready_tasks) => {
                        for task in ready_tasks {
                            let handlers_guard = handlers.read().await;

                            if let Some(handler) = handlers_guard.get(&task.id) {
                                let handler = Arc::clone(handler);
                                let storage_clone = Arc::clone(&storage);

                                tokio::spawn(async move {
                                    Self::execute_task_with_retry(handler, task, storage_clone)
                                        .await;
                                });
                            } else {
                                eprintln!("No handler registered for task {}", task.id);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error fetching ready tasks: {:?}", e);
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn stop(&self) -> Result<(), SchedulerError> {
        let mut running = self.running.write().await;
        if !*running {
            return Err(SchedulerError::NotRunning);
        }
        *running = false;
        Ok(())
    }
}
