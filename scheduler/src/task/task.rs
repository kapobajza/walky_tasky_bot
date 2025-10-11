use std::{collections::HashMap, str::FromStr};

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::error::SchedulerError;

#[derive(Clone, Debug)]
pub struct Task {
    pub id: Uuid,
    pub cron_expression: Option<String>,
    pub next_run: DateTime<Utc>,
    pub last_run: Option<DateTime<Utc>>,
    pub enabled: bool,
    pub metadata: HashMap<String, String>,
    once: bool,
}

impl Task {
    pub fn new_with_cron(cron_expression: &str) -> Result<Self, SchedulerError> {
        let id = uuid::Uuid::new_v4();
        let next_run = cron::Schedule::from_str(cron_expression)
            .map_err(|error| SchedulerError::CronError(error))?
            .upcoming(Utc)
            .next()
            .ok_or_else(|| SchedulerError::NoChronoNext)?;

        Ok(Task {
            id,
            cron_expression: Some(cron_expression.to_string()),
            next_run,
            last_run: None,
            enabled: true,
            metadata: HashMap::new(),
            once: false,
        })
    }

    pub fn new_with_datetime(next_run: DateTime<Utc>) -> Self {
        let id = uuid::Uuid::new_v4();
        Task {
            id,
            cron_expression: None,
            next_run,
            last_run: None,
            enabled: true,
            metadata: HashMap::new(),
            once: true,
        }
    }

    pub fn calculate_next_run(&mut self) -> Result<(), SchedulerError> {
        if let Some(cron_expr) = &self.cron_expression {
            let schedule = cron::Schedule::from_str(cron_expr)
                .map_err(|error| SchedulerError::CronError(error))?;
            self.next_run = schedule
                .upcoming(Utc)
                .next()
                .ok_or_else(|| SchedulerError::NoChronoNext)?;
        }

        if self.once {
            self.enabled = false;
        }

        Ok(())
    }
}
