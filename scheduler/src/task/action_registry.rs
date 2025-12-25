use crate::{
    error::SchedulerError,
    task::{action::TaskAction, action_executor::ActionExecutor, default::Task},
};

pub struct ActionRegistry {
    executors: Vec<Box<dyn ActionExecutor>>,
}

impl Default for ActionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionRegistry {
    pub fn new() -> Self {
        Self {
            executors: Vec::new(),
        }
    }

    pub fn register(&mut self, executor: impl ActionExecutor + 'static) {
        self.executors.push(Box::new(executor));
    }

    pub async fn execute(&self, task: &Task) -> Result<(), SchedulerError> {
        for executor in &self.executors {
            match &task.action {
                Some(action) => {
                    if self.can_execute(action, executor) {
                        return executor.execute(task, action).await;
                    }
                }
                None => return Err(SchedulerError::ActionMissing(task.id.to_string())),
            }
        }

        Ok(())
    }

    pub fn has_executor_for(&self, action: &TaskAction) -> bool {
        for executor in &self.executors {
            if self.can_execute(action, executor) {
                return true;
            }
        }
        false
    }

    fn can_execute(&self, action: &TaskAction, executor: &Box<dyn ActionExecutor>) -> bool {
        executor.supported_actions().contains(&action.action_type())
    }
}
