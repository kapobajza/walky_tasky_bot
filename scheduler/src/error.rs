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
}
