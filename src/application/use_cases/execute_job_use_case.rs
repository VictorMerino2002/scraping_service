use std::sync::Arc;
use std::time::Duration;

use uuid::Uuid;

use crate::{
    application::ports::{JobRepositoryPort, ScraperPort},
    domain::value_objects::Error,
};

// Comfortably below the executor Lambda's configured 300s timeout (see
// serverless.yml). A dead CDP/WebSocket connection to the browser doesn't
// always surface as an error from the scraper (see the WS handling in
// ChromiumoxideScraper) — without an upper bound here, that kind of hang
// burns the full 5 minutes and AWS force-kills the function before
// `job.fail()` ever runs, leaving the job stuck at "InProgress" forever.
const EXECUTION_TIMEOUT: Duration = Duration::from_secs(270);

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

        match tokio::time::timeout(EXECUTION_TIMEOUT, self.scraper.execute(&job)).await {
            Ok(Ok(results)) => job.complete(results)?,
            Ok(Err(error)) => job.fail(error.to_string())?,
            Err(_) => job.fail(format!(
                "Job execution timed out after {EXECUTION_TIMEOUT:?} \
                 (the browser/CDP connection likely became unresponsive)"
            ))?,
        }

        self.job_repository.save(&job).await
    }
}
