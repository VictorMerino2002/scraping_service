use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::ports::{ActionSerializer, JobRepositoryPort},
    domain::{
        entities::Job,
        value_objects::{Error, JobConfig},
    },
};

pub struct CreateJobUseCase {
    pub action_serializer: Arc<dyn ActionSerializer<Input = String> + Send + Sync>,
    pub job_repository: Arc<dyn JobRepositoryPort>,
}

impl CreateJobUseCase {
    pub async fn execute(&self, actions: String, config: Option<JobConfig>) -> Result<Uuid, Error> {
        let actions = self.action_serializer.from(actions)?;
        let job = Job::new(actions, config)?;

        self.job_repository.save(&job).await?;

        Ok(job.id())
    }
}
