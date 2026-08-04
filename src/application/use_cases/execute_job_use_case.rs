use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::ports::{JobRepositoryPort, ScraperPort},
    domain::value_objects::Error,
};

pub struct ExecuteJobUseCase {
    pub scraper: Arc<dyn ScraperPort>,
    pub job_repository: Arc<dyn JobRepositoryPort>,
}

impl ExecuteJobUseCase {
    pub async fn execute(&self, job_id: &Uuid) -> Result<(), Error> {
        let mut job = self
            .job_repository
            .get_by_id(job_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Job with ID {job_id} not found")))?;

        job.start()?;
        self.job_repository.save(&job).await?;

        match self.scraper.execute(&job).await {
            Ok(_) => job.complete()?,
            Err(error) => job.fail(error.to_string())?,
        }

        self.job_repository.save(&job).await
    }
}
