use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{entities::Job, value_objects::Error};

#[async_trait]
pub trait JobRepositoryPort: Send + Sync {
    async fn save(&self, job: &Job) -> Result<(), Error>;
    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Job>, Error>;
    async fn delete(&self, id: &Uuid) -> Result<(), Error>;
}
