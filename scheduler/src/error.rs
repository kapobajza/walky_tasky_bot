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

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}
