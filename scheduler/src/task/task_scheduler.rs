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

    pub async fn add_task_with_cron(
        &self,
        cron_expression: &str,
        handler: Arc<dyn TaskHandler>,
    ) -> Result<Uuid, SchedulerError> {
        let task = Task::new_with_cron(cron_expression)?;

        self.storage.save_task(task.clone()).await?;

        let mut handlers = self.handlers.write().await;
        handlers.insert(task.id, handler);

        Ok(task.id)
    }

    pub async fn add_task_with_datetime(
        &self,
        next_run: chrono::DateTime<chrono::Utc>,
        handler: Arc<dyn TaskHandler>,
    ) -> Result<Uuid, SchedulerError> {
        let task = Task::new_with_datetime(next_run);

        self.storage.save_task(task.clone()).await?;

        let mut handlers = self.handlers.write().await;
        handlers.insert(task.id, handler);
        println!("Added task with ID: {}", task.id);

        Ok(task.id)
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
                        for mut task in ready_tasks {
                            let handlers_guard = handlers.read().await;
                            println!("Running task: {}", task.id);

                            if let Some(handler) = handlers_guard.get(&task.id) {
                                let handler = Arc::clone(handler);
                                let task_clone = task.clone();

                                tokio::spawn(async move {
                                    match handler.execute(&task_clone).await {
                                        Ok(_) => {
                                            println!(
                                                "Task {} executed successfully",
                                                task_clone.id
                                            );
                                        }
                                        Err(e) => {
                                            eprintln!(
                                                "Error executing task {}: {:?}",
                                                task_clone.id, e
                                            );
                                        }
                                    }
                                });

                                task.last_run = Some(chrono::Utc::now());

                                if let Err(e) = task.calculate_next_run() {
                                    eprintln!(
                                        "Error calculating next run for task {}: {:?}",
                                        task.id, e
                                    );
                                    continue;
                                }

                                if let Err(e) = storage.save_task(task).await {
                                    eprintln!("Error updating task {:?}", e);
                                }
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
