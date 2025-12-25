use async_trait::async_trait;

use crate::{
    error::SchedulerError,
    task::{
        action::{ActionType, TaskAction},
        default::Task,
    },
};

#[async_trait]
pub trait ActionExecutor: Send + Sync {
    fn supported_actions(&self) -> Vec<ActionType>;
    async fn execute(&self, task: &Task, action: &TaskAction) -> Result<(), SchedulerError>;
}

pub type BoxedActionExecutor = Box<dyn ActionExecutor>;
