use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::types::{JsonValue, time::OffsetDateTime};
use uuid::Uuid;

use crate::{error::SchedulerError, task::action::TaskAction};

#[derive(Clone, Debug)]
pub enum TaskType {
    Once,
    Range {
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    },
}

impl From<TaskType> for i16 {
    fn from(value: TaskType) -> Self {
        match value {
            TaskType::Once => 1,
            TaskType::Range { .. } => 2,
        }
    }
}

pub struct TaskDb {
    pub id: Uuid,
    pub schedule_type: i16,
    pub last_run: Option<OffsetDateTime>,
    pub next_run: OffsetDateTime,
    pub retry_count: i32,
    pub max_retries: i32,
    pub retry_delay: i32,
    pub enabled: bool,
    pub action: JsonValue,
    pub start_date: Option<OffsetDateTime>,
    pub end_date: Option<OffsetDateTime>,
}

fn to_offset_datetime(dt: DateTime<Utc>) -> Result<OffsetDateTime, SchedulerError> {
    OffsetDateTime::from_unix_timestamp(dt.timestamp())
        .map_err(|e| SchedulerError::DatabaseError(e.to_string()))
}

fn from_offset_datetime(odt: OffsetDateTime) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp_nanos(odt.unix_timestamp_nanos() as i64)
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
    pub delay_between_runs: Option<chrono::Duration>,
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
            delay_between_runs: None,
        }
    }
}

impl Task {
    pub fn new_with_datetime_range(
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        action: TaskAction,
    ) -> Self {
        Task {
            schedule: TaskType::Range {
                start_date,
                end_date,
            },
            next_run: start_date,
            action: Some(action),
            ..Default::default()
        }
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

    pub fn with_delay_between_runs(mut self, delay: chrono::Duration) -> Self {
        self.delay_between_runs = Some(delay);
        self
    }

    pub fn calculate_next_run(&mut self) {
        match &self.schedule {
            TaskType::Range {
                start_date: _,
                end_date,
            } => {
                let next_run =
                    self.next_run + self.delay_between_runs.unwrap_or(chrono::Duration::days(1));

                if next_run <= *end_date {
                    self.next_run = next_run;
                } else {
                    self.enabled = false;
                }
            }
            TaskType::Once => {
                self.enabled = false;
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
        let next_run = to_offset_datetime(self.next_run)?;
        let last_run = self.last_run.map(to_offset_datetime).transpose()?;
        let action = match &self.action {
            Some(act) => act,
            None => {
                return Err(SchedulerError::ActionMissing(self.id.to_string()));
            }
        };

        let (start_date, end_date) = match &self.schedule {
            TaskType::Range {
                start_date,
                end_date,
            } => {
                let start_date = to_offset_datetime(*start_date)?;
                let end_date = to_offset_datetime(*end_date)?;

                (Some(start_date), Some(end_date))
            }
            _ => (None, None),
        };

        Ok(TaskDb {
            id: self.id,
            schedule_type: schedule,
            last_run,
            next_run,
            retry_count,
            max_retries,
            retry_delay,
            enabled: self.enabled,
            action: serde_json::to_value(action)?,
            start_date,
            end_date,
        })
    }

    pub fn from_db_task(db_task: TaskDb) -> Result<Self, SchedulerError> {
        let schedule = match db_task.schedule_type {
            1 => TaskType::Once,
            2 => TaskType::Range {
                start_date: from_offset_datetime(db_task.start_date.ok_or_else(|| {
                    SchedulerError::DatabaseError("Missing start_date for Range task".to_string())
                })?),
                end_date: from_offset_datetime(db_task.end_date.ok_or_else(|| {
                    SchedulerError::DatabaseError("Missing end_date for Range task".to_string())
                })?),
            },
            _ => {
                return Err(SchedulerError::DatabaseError(
                    "Invalid schedule type".to_string(),
                ));
            }
        };
        let retry_delay = Duration::from_millis(db_task.retry_delay as u64);
        let max_retries = db_task.max_retries as u32;
        let retry_count = db_task.retry_count as u32;
        let next_run = from_offset_datetime(db_task.next_run);
        let last_run = db_task.last_run.map(from_offset_datetime);
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
            delay_between_runs: None,
        })
    }
}
