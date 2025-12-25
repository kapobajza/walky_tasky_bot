use async_trait::async_trait;

use crate::{
    error::SchedulerError,
    task::{
        action::{ActionType, TaskAction},
        action_executor::ActionExecutor,
        default::Task,
    },
};

pub struct LogExecutor;

impl Default for LogExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl LogExecutor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ActionExecutor for LogExecutor {
    fn supported_actions(&self) -> Vec<ActionType> {
        vec![ActionType::Log]
    }

    async fn execute(&self, task: &Task, action: &TaskAction) -> Result<(), SchedulerError> {
        if let TaskAction::Log { message, level } = action {
            match level.as_str() {
                "info" => log::info!("[Task {}] {}", task.id, message),
                "warn" => log::warn!("[Task {}] {}", task.id, message),
                "error" => log::error!("[Task {}] {}", task.id, message),
                "debug" => log::debug!("[Task {}] {}", task.id, message),
                _ => {
                    log::warn!(
                        "[Task {}] Unknown log level '{}', with message: {}",
                        task.id,
                        level,
                        message
                    );
                }
            }
            Ok(())
        } else {
            Err(SchedulerError::UnsupportedAction)
        }
    }
}
