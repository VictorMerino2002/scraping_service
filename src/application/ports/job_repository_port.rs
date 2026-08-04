use uuid::Uuid;

use crate::domain::{entities::Job, value_objects::Error};

pub trait JobRepositoryPort {
    async fn save(&self, job: &Job) -> Result<(), Error>;
    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Job>, Error>;
    async fn delete(&self, id: &Uuid) -> Result<(), Error>;
}
