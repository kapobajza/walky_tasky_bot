use crate::{error::SchedulerError, task::task::Task};

#[async_trait::async_trait]
pub trait TaskHandler: Send + Sync {
    async fn execute(&self, task: &Task) -> Result<(), SchedulerError>;
}

pub struct AsyncTaskHandler<F, Fut>
where
    F: Fn(Task) -> Fut + Send + Sync,
    Fut: Future<Output = Result<(), SchedulerError>> + Send,
{
    handler: F,
}

impl<F, Fut> AsyncTaskHandler<F, Fut>
where
    F: Fn(Task) -> Fut + Send + Sync,
    Fut: Future<Output = Result<(), SchedulerError>> + Send,
{
    pub fn new(handler: F) -> Self {
        Self { handler }
    }
}

#[async_trait::async_trait]
impl<F, Fut> TaskHandler for AsyncTaskHandler<F, Fut>
where
    F: Fn(Task) -> Fut + Send + Sync,
    Fut: Future<Output = Result<(), SchedulerError>> + Send,
{
    async fn execute(&self, task: &Task) -> Result<(), SchedulerError> {
        (self.handler)(task.clone()).await
    }
}

pub struct SimpleTaskHandler<F>
where
    F: Fn(&Task) -> Result<(), SchedulerError> + Send + Sync,
{
    handler: F,
}

#[async_trait::async_trait]
impl<F> TaskHandler for SimpleTaskHandler<F>
where
    F: Fn(&Task) -> Result<(), SchedulerError> + Send + Sync,
{
    async fn execute(&self, task: &Task) -> Result<(), SchedulerError> {
        (self.handler)(task)
    }
}

impl<F> SimpleTaskHandler<F>
where
    F: Fn(&Task) -> Result<(), SchedulerError> + Send + Sync,
{
    pub fn new(handler: F) -> Self {
        Self { handler }
    }
}
