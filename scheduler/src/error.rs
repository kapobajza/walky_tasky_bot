use thiserror::Error;

#[derive(Debug, Error)]
pub enum SchedulerError {
    #[error("Cron error: {0}")]
    CronError(#[from] cron::error::Error),

    #[error("No next occurrence found in cron schedule")]
    NoChronoNext,

    #[error("Scheduler is already running")]
    AlreadyRunning,

    #[error("Scheduler is not running")]
    NotRunning,

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Task execution error: {0}")]
    TaskExecutionError(String),

    #[error("Migration error: {0}")]
    MigrationError(String),

    #[error("Unsupported action type")]
    UnsupportedAction,

    #[error("Action missing for the task {0}")]
    ActionMissing(String),

    #[error("Action not found in registry")]
    RegistryActionNotFound,

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization/Deserialization error: {0}")]
    SerdeError(#[from] serde_json::Error),
}
