use std::{str::FromStr, time::Duration};

use chrono::{DateTime, Utc};
use sqlx::types::{JsonValue, time::OffsetDateTime};
use uuid::Uuid;

use crate::{error::SchedulerError, task::action::TaskAction};

#[derive(Clone, Debug)]
pub enum TaskType {
    Cron(String),
    Once,
}

impl From<TaskType> for i16 {
    fn from(value: TaskType) -> Self {
        match value {
            TaskType::Cron(_) => 1,
            TaskType::Once => 2,
        }
    }
}

pub struct TaskDb {
    pub id: Uuid,
    pub schedule_type: i16,
    pub schedule: Option<String>,
    pub last_run: Option<OffsetDateTime>,
    pub next_run: OffsetDateTime,
    pub retry_count: i32,
    pub max_retries: i32,
    pub retry_delay: i32,
    pub enabled: bool,
    pub action: JsonValue,
}

#[derive(Clone, Debug)]
pub struct Task {
    pub id: Uuid,
    pub next_run: DateTime<Utc>,
    pub last_run: Option<DateTime<Utc>>,
    pub enabled: bool,
    pub retry_count: u32,
    pub max_retries: u32,
    pub retry_delay: Duration,
    pub schedule: TaskType,
    pub action: Option<TaskAction>,
}

impl Default for Task {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            next_run: Utc::now(),
            last_run: None,
            enabled: true,
            retry_count: 0,
            max_retries: 3,
            retry_delay: Duration::from_millis(1000),
            schedule: TaskType::Once,
            action: None,
        }
    }
}

impl Task {
    pub fn new_with_cron(
        cron_expression: &str,
        action: TaskAction,
    ) -> Result<Self, SchedulerError> {
        let next_run = cron::Schedule::from_str(cron_expression)
            .map_err(SchedulerError::CronError)?
            .upcoming(Utc)
            .next()
            .ok_or(SchedulerError::NoChronoNext)?;

        Ok(Task {
            schedule: TaskType::Cron(cron_expression.to_string()),
            next_run,
            action: Some(action),
            ..Default::default()
        })
    }

    pub fn new_with_datetime(next_run: DateTime<Utc>, action: TaskAction) -> Self {
        Task {
            schedule: TaskType::Once,
            next_run,
            action: Some(action),
            ..Default::default()
        }
    }

    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn with_retry_delay(mut self, retry_delay: Duration) -> Self {
        self.retry_delay = retry_delay;
        self
    }

    pub fn calculate_next_run(&mut self) -> Result<(), SchedulerError> {
        match &self.schedule {
            TaskType::Cron(cron_expression) => {
                let schedule =
                    cron::Schedule::from_str(cron_expression).map_err(SchedulerError::CronError)?;
                let next_run = schedule
                    .upcoming(Utc)
                    .next()
                    .ok_or(SchedulerError::NoChronoNext)?;
                self.next_run = next_run;
                Ok(())
            }
            TaskType::Once => {
                self.enabled = false;
                Ok(())
            }
        }
    }

    pub fn calcluate_retry_delay(&self) -> Duration {
        let multiplier = 2_u64.pow(self.retry_count);
        Duration::from_millis(self.retry_delay.as_millis() as u64 * multiplier)
    }

    pub fn should_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    pub fn reset_retry_count(&mut self) {
        self.retry_count = 0;
    }

    pub fn to_db_task(&self) -> Result<TaskDb, SchedulerError> {
        let schedule = i16::from(self.schedule.clone());
        let retry_delay = self.retry_delay.as_millis() as i32;
        let max_retries = self.max_retries as i32;
        let retry_count = self.retry_count as i32;
        let next_run = OffsetDateTime::from_unix_timestamp(self.next_run.timestamp())
            .map_err(|e| SchedulerError::DatabaseError(e.to_string()))?;
        let last_run = self
            .last_run
            .map(|dt| OffsetDateTime::from_unix_timestamp(dt.timestamp()))
            .transpose()
            .map_err(|e| SchedulerError::DatabaseError(e.to_string()))?;
        let action = match &self.action {
            Some(act) => act,
            None => {
                return Err(SchedulerError::ActionMissing(self.id.to_string()));
            }
        };

        Ok(TaskDb {
            id: self.id,
            schedule_type: schedule,
            last_run,
            next_run,
            retry_count,
            max_retries,
            retry_delay,
            schedule: match &self.schedule {
                TaskType::Cron(expr) => Some(expr.clone()),
                TaskType::Once => None,
            },
            enabled: self.enabled,
            action: serde_json::to_value(action)?,
        })
    }

    pub fn from_db_task(db_task: TaskDb) -> Result<Self, SchedulerError> {
        let schedule = match db_task.schedule_type {
            1 => TaskType::Cron(db_task.schedule.unwrap()), // Placeholder, actual cron expression should be stored/retrieved
            2 => TaskType::Once,
            _ => {
                return Err(SchedulerError::DatabaseError(
                    "Invalid schedule type".to_string(),
                ));
            }
        };
        let retry_delay = Duration::from_millis(db_task.retry_delay as u64);
        let max_retries = db_task.max_retries as u32;
        let retry_count = db_task.retry_count as u32;
        let next_run =
            DateTime::<Utc>::from_timestamp_nanos(db_task.next_run.unix_timestamp_nanos() as i64);
        let last_run = db_task
            .last_run
            .map(|dt| DateTime::<Utc>::from_timestamp_nanos(dt.unix_timestamp_nanos() as i64));
        let action: TaskAction = serde_json::from_value(db_task.action)?;

        Ok(Task {
            id: db_task.id,
            schedule,
            next_run,
            last_run,
            enabled: db_task.enabled,
            retry_count,
            max_retries,
            retry_delay,
            action: Some(action),
        })
    }
}
