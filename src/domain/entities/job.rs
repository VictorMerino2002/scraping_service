use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::value_objects::{Error, JobAction, JobConfig, JobStatus};

pub struct Job {
    id: Uuid,
    actions: Vec<JobAction>,
    status: JobStatus,
    config: Option<JobConfig>,
    error: Option<String>,
    created_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
}

impl Job {
    pub fn new(actions: Vec<JobAction>, config: Option<JobConfig>) -> Result<Self, Error> {
        if actions.is_empty() {
            return Err(Error::InvalidInput(
                "a job must have at least one action".to_string(),
            ));
        }

        Ok(Self {
            id: Uuid::new_v4(),
            actions,
            status: JobStatus::Pending,
            config,
            error: None,
            created_at: Utc::now(),
            started_at: None,
            finished_at: None,
        })
    }

    /// Rebuilds a `Job` from previously persisted state, bypassing the invariants
    /// enforced by `new`. Intended for repository adapters that rehydrate a job
    /// that was already valid when it was originally created.
    #[allow(clippy::too_many_arguments)]
    pub fn from_persisted(
        id: Uuid,
        actions: Vec<JobAction>,
        status: JobStatus,
        config: Option<JobConfig>,
        error: Option<String>,
        created_at: DateTime<Utc>,
        started_at: Option<DateTime<Utc>>,
        finished_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            actions,
            status,
            config,
            error,
            created_at,
            started_at,
            finished_at,
        }
    }

    pub fn start(&mut self) -> Result<(), Error> {
        if self.status != JobStatus::Pending {
            return Err(Error::InvalidInput(format!(
                "cannot start a job in status {:?}",
                self.status
            )));
        }

        self.status = JobStatus::InProgress;
        self.started_at = Some(Utc::now());
        Ok(())
    }

    pub fn complete(&mut self) -> Result<(), Error> {
        if self.status != JobStatus::InProgress {
            return Err(Error::InvalidInput(format!(
                "cannot complete a job in status {:?}",
                self.status
            )));
        }

        self.status = JobStatus::Completed;
        self.finished_at = Some(Utc::now());
        Ok(())
    }

    pub fn fail(&mut self, error: String) -> Result<(), Error> {
        if self.status != JobStatus::InProgress {
            return Err(Error::InvalidInput(format!(
                "cannot fail a job in status {:?}",
                self.status
            )));
        }

        self.status = JobStatus::Failed;
        self.error = Some(error);
        self.finished_at = Some(Utc::now());
        Ok(())
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn actions(&self) -> &[JobAction] {
        &self.actions
    }

    pub fn status(&self) -> JobStatus {
        self.status
    }

    pub fn config(&self) -> Option<&JobConfig> {
        self.config.as_ref()
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn started_at(&self) -> Option<DateTime<Utc>> {
        self.started_at
    }

    pub fn finished_at(&self) -> Option<DateTime<Utc>> {
        self.finished_at
    }
}
