use std::{
    sync::Arc,
    time::{self, Duration},
};

use crate::{
    db::migrator::Migrator,
    storage::{
        base_storage::Storage, database_storage::DatabaseStorage,
        in_memory_storage::InMemoryStorage,
    },
    task::{
        task::{Task, TaskType},
        task_handler::{AsyncTaskHandler, SimpleTaskHandler},
        task_scheduler::TaskScheduler,
    },
};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres as PostgresImage;

static DB_NAME: &str = "test_db";

async fn setup_database() -> (sqlx::Pool<sqlx::Postgres>, ContainerAsync<PostgresImage>) {
    let db_user = "postgres";
    let db_password = "postgres";
    let pg_container = PostgresImage::default()
        .with_env_var("POSTGRES_USER", db_user)
        .with_env_var("POSTGRES_PASSWORD", db_password)
        .with_env_var("POSTGRES_DB", DB_NAME)
        .start()
        .await
        .expect("Failed to start Postgres container");

    let port = pg_container
        .get_host_port_ipv4(5432)
        .await
        .expect("Failed to get host port");

    let database_url = format!(
        "postgres://{}:{}@localhost:{}/{}",
        db_user, db_password, port, DB_NAME
    );

    let pool = sqlx::Pool::<sqlx::Postgres>::connect(&database_url)
        .await
        .expect("Failed to connect to the database");

    sqlx::query("CREATE EXTENSION IF NOT EXISTS pgcrypto;")
        .execute(&pool)
        .await
        .expect("Failed to enable pgcrypto extension");

    Migrator::run(&database_url)
        .await
        .expect("Failed to run migrations");

    (pool, pg_container)
}

async fn setup_db_storage(container: &ContainerAsync<PostgresImage>) -> Arc<DatabaseStorage> {
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("Failed to get host port");

    let database_url = format!(
        "postgres://{}:{}@localhost:{}/{}",
        "postgres", "postgres", port, DB_NAME
    );

    Arc::new(
        DatabaseStorage::new(&database_url)
            .await
            .expect("Failed to create DatabaseStorage"),
    )
}

async fn get_run_tasks<S: Storage + ?Sized>(storage: &Arc<S>) -> usize {
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
        .add_task(
            Task::new_with_datetime("test_add_execute", next_run),
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

#[tokio::test]
async fn test_retry_task_on_failure() {
    let storage = Arc::new(InMemoryStorage::new());
    let scheduler =
        TaskScheduler::new(storage.clone()).with_check_interval(time::Duration::from_millis(50));
    let now = chrono::Utc::now();
    let next_run = now + chrono::Duration::milliseconds(10);
    let attempt_counter = Arc::new(tokio::sync::Mutex::new(0));
    let attempt_counter_clone = attempt_counter.clone();

    scheduler
        .add_task(
            Task::new_with_datetime("test_retry_on_failure", next_run)
                .with_max_retries(3)
                .with_retry_delay(Duration::from_millis(10)),
            Arc::new(AsyncTaskHandler::new(move |_| {
                let counter = attempt_counter_clone.clone();
                async move {
                    let mut count = counter.lock().await;
                    *count += 1;
                    let current_attempt = *count;
                    drop(count);

                    if current_attempt < 3 {
                        println!("Simulated failure for attempt {}", current_attempt);
                        Err(crate::error::SchedulerError::TaskExecutionError(
                            "Simulated failure".into(),
                        ))
                    } else {
                        Ok(())
                    }
                }
            })),
        )
        .await
        .unwrap();

    let run_tasks = get_run_tasks(&storage).await;
    assert_eq!(run_tasks, 0);

    scheduler.start().await.unwrap();

    tokio::time::sleep(Duration::from_millis(300)).await;

    let run_tasks = get_run_tasks(&storage).await;
    assert_eq!(run_tasks, 1);

    let final_attempts = *attempt_counter.lock().await;
    assert_eq!(final_attempts, 4);
}

#[tokio::test]
async fn test_execute_unfinished_tasks_on_startup() {
    let (_pool, container) = setup_database().await;
    let storage = setup_db_storage(&container).await;
    let scheduler =
        TaskScheduler::new(storage.clone()).with_check_interval(time::Duration::from_millis(50));
    let now = chrono::Utc::now();
    let next_run = now - chrono::Duration::days(1);
    scheduler
        .add_task(
            Task::new_with_datetime("test_execute_unfinished_tasks_on_startup", next_run),
            Arc::new(SimpleTaskHandler::new(|task| {
                println!("Task executed on startup: {:?}", task);
                Ok(())
            })),
        )
        .await
        .unwrap();

    let run_tasks = get_run_tasks(&storage).await;
    assert_eq!(run_tasks, 0);

    scheduler.start().await.unwrap();

    tokio::time::sleep(Duration::from_millis(200)).await;

    let run_tasks = get_run_tasks(&storage).await;
    assert_eq!(run_tasks, 1);
}

#[tokio::test]
async fn test_do_not_execute_disabled_tasks() {
    let (_pool, container) = setup_database().await;
    let storage = setup_db_storage(&container).await;
    let scheduler =
        TaskScheduler::new(storage.clone()).with_check_interval(time::Duration::from_millis(50));
    let now = chrono::Utc::now();

    let task_name = "test_do_not_execute_disabled_tasks";

    storage
        .save_task(Task {
            id: uuid::Uuid::new_v4(),
            name: task_name.to_string(),
            schedule: TaskType::Once,
            next_run: now - chrono::Duration::days(1),
            last_run: None,
            retry_count: 0,
            max_retries: 3,
            retry_delay: Duration::from_millis(10),
            enabled: false,
        })
        .await
        .unwrap();

    let run_tasks = get_run_tasks(&storage).await;
    assert_eq!(run_tasks, 0);

    scheduler.start().await.unwrap();

    tokio::time::sleep(Duration::from_millis(200)).await;

    let run_tasks = get_run_tasks(&storage).await;
    assert_eq!(run_tasks, 0);
}
