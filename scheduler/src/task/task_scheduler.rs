use std::{collections::HashSet, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    error::SchedulerError,
    storage::base_storage::Storage,
    task::{action_registry::ActionRegistry, default::Task},
};

#[derive(Clone)]
pub struct TaskScheduler {
    storage: Arc<dyn Storage>,
    action_registry: Arc<ActionRegistry>,
    running: Arc<RwLock<bool>>,
    check_interval: Duration,
    executing_tasks: Arc<RwLock<HashSet<Uuid>>>,
}

impl TaskScheduler {
    pub fn new(storage: Arc<dyn Storage>, registry: ActionRegistry) -> Self {
        Self {
            storage,
            action_registry: Arc::new(registry),
            running: Arc::new(RwLock::new(false)),
            check_interval: Duration::from_millis(500),
            executing_tasks: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    pub fn with_check_interval(mut self, interval: Duration) -> Self {
        self.check_interval = interval;
        self
    }

    pub async fn add_task(&self, task: Task) -> Result<Uuid, SchedulerError> {
        let action = match &task.action {
            Some(act) => act,
            None => {
                return Err(SchedulerError::ActionMissing(task.id.to_string()));
            }
        };

        if !self.action_registry.has_executor_for(action) {
            return Err(SchedulerError::RegistryActionNotFound);
        }

        self.storage.save_task(task.clone()).await?;
        Ok(task.id)
    }

    async fn execute_task_with_retry(
        registry: Arc<ActionRegistry>,
        mut task: Task,
        storage: Arc<dyn Storage>,
        executing_tasks: Arc<RwLock<HashSet<Uuid>>>,
    ) {
        loop {
            match registry.execute(&task).await {
                Ok(_) => {
                    log::info!("Task {} executed successfully", task.id);
                    task.reset_retry_count();
                    task.last_run = Some(chrono::Utc::now());

                    let mut executing_guard = executing_tasks.write().await;
                    executing_guard.remove(&task.id);

                    task.calculate_next_run();

                    if let Err(e) = storage.save_task(task).await {
                        log::error!("Error updating task {:?}", e);
                    }

                    return;
                }
                Err(e) => {
                    task.retry_count += 1;
                    log::error!(
                        "Error executing task {}: {:?}. Retry count: {}",
                        task.id,
                        e,
                        task.retry_count
                    );

                    if task.should_retry() {
                        let retry_delay = task.calcluate_retry_delay();
                        tokio::time::sleep(retry_delay).await;
                        continue;
                    } else {
                        log::error!("Max retries reached for task {}. Giving up.", task.id);
                        task.last_run = Some(chrono::Utc::now());
                        task.calculate_next_run();
                        task.reset_retry_count();

                        if let Err(e) = storage.save_task(task).await {
                            log::error!("Error updating task {:?}", e);
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
        let running = Arc::clone(&self.running);
        let check_interval = self.check_interval;
        let executing_tasks = Arc::clone(&self.executing_tasks);
        let registry = Arc::clone(&self.action_registry);

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
                            let mut executing_guard = executing_tasks.write().await;

                            if executing_guard.contains(&task.id) {
                                continue;
                            } else {
                                executing_guard.insert(task.id);
                            }

                            drop(executing_guard);

                            let storage_clone = Arc::clone(&storage);
                            let executing_tasks = Arc::clone(&executing_tasks);
                            let registry = Arc::clone(&registry);

                            tokio::spawn(async move {
                                Self::execute_task_with_retry(
                                    registry,
                                    task,
                                    storage_clone,
                                    executing_tasks,
                                )
                                .await;
                            });
                        }
                    }
                    Err(e) => {
                        log::error!("Error fetching ready tasks: {:?}", e);
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

    pub fn shutdown_on_ctrl_c(&self) -> tokio::task::JoinHandle<Result<(), SchedulerError>> {
        let running = Arc::clone(&self.running);
        tokio::spawn(async move {
            tokio::signal::ctrl_c().await?;
            log::info!("Ctrl-C received, shutting down scheduler...");
            let mut running_guard = running.write().await;
            *running_guard = false;
            Ok(())
        })
    }
}
