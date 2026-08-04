use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::ports::JobRepositoryPort,
    domain::{entities::Job, value_objects::Error},
};

pub struct GetJobUseCase {
    pub job_repository: Arc<dyn JobRepositoryPort>,
}

impl GetJobUseCase {
    pub async fn execute(&self, job_id: &Uuid) -> Result<Job, Error> {
        self.job_repository
            .get_by_id(job_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Job with ID {job_id} not found")))
    }
}
