use async_trait::async_trait;
use scheduler::{
    error::SchedulerError,
    storage::in_memory_storage::InMemoryStorage,
    task::{
        action::{ActionType, TaskAction},
        action_executor::ActionExecutor,
        action_registry::ActionRegistry,
        default::Task,
        task_scheduler::TaskScheduler,
    },
};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Test executor that captures sent bot messages instead of actually sending them.
/// Follows the CountingExecutor pattern from scheduler tests.
pub struct CapturingBotExecutor {
    pub captured_messages: Arc<Mutex<Vec<(i64, String)>>>,
}

impl CapturingBotExecutor {
    pub fn new(captured: Arc<Mutex<Vec<(i64, String)>>>) -> Self {
        Self {
            captured_messages: captured,
        }
    }
}

#[async_trait]
impl ActionExecutor for CapturingBotExecutor {
    fn supported_actions(&self) -> Vec<ActionType> {
        vec![ActionType::SendBotMessage]
    }

    async fn execute(&self, _task: &Task, action: &TaskAction) -> Result<(), SchedulerError> {
        if let TaskAction::SendBotMessage { chat_id, message } = action {
            let mut messages = self.captured_messages.lock().await;
            messages.push((*chat_id, message.clone()));
        }
        Ok(())
    }
}

/// Creates a test scheduler with in-memory storage that returns the storage handle
/// for direct task verification.
#[allow(clippy::type_complexity)]
pub fn create_test_scheduler_with_storage() -> (
    TaskScheduler,
    Arc<InMemoryStorage>,
    Arc<Mutex<Vec<(i64, String)>>>,
) {
    let storage = Arc::new(InMemoryStorage::new());
    let captured = Arc::new(Mutex::new(Vec::new()));

    let mut registry = ActionRegistry::new();
    registry.register(CapturingBotExecutor::new(captured.clone()));

    let scheduler = TaskScheduler::new(storage.clone(), registry)
        .with_check_interval(std::time::Duration::from_millis(50));

    (scheduler, storage, captured)
}
