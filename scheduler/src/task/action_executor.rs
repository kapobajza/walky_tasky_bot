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

#[async_trait]
impl ActionExecutor for Box<dyn ActionExecutor> {
    fn supported_actions(&self) -> Vec<ActionType> {
        self.as_ref().supported_actions()
    }

    async fn execute(&self, task: &Task, action: &TaskAction) -> Result<(), SchedulerError> {
        self.as_ref().execute(task, action).await
    }
}
