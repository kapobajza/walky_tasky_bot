use std::{
    sync::Arc,
    time::{self, Duration},
};

use crate::{
    storage::{base_storage::Storage, in_memory_storage::InMemoryStorage},
    task::{task_handler::SimpleTaskHandler, task_scheduler::TaskScheduler},
};

async fn get_run_tasks(storage: &Arc<InMemoryStorage>) -> usize {
    storage
        .get_all_tasks()
        .await
        .unwrap()
        .iter()
        .filter(|t| t.last_run.is_some())
        .count()
}

#[tokio::test]
async fn test_add_and_execute_task() {
    let storage = Arc::new(InMemoryStorage::new());
    let scheduler =
        TaskScheduler::new(storage.clone()).with_check_interval(time::Duration::from_millis(50));
    let now = chrono::Utc::now();
    let next_run = now + chrono::Duration::milliseconds(10);

    scheduler
        .add_task_with_datetime(
            next_run,
            Arc::new(SimpleTaskHandler::new(|task| {
                println!("Task executed: {:?}", task);
                Ok(())
            })),
        )
        .await
        .unwrap();

    let run_tasks = get_run_tasks(&storage).await;

    assert_eq!(run_tasks, 0);

    scheduler.start().await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let run_tasks = get_run_tasks(&storage).await;
    assert_eq!(run_tasks, 1);
}
