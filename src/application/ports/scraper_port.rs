use std::collections::HashMap;

use async_trait::async_trait;

use crate::domain::entities::Job;
use crate::domain::value_objects::Error;

#[async_trait]
pub trait ScraperPort: Send + Sync {
    async fn execute(&self, job: &Job) -> Result<HashMap<String, String>, Error>;
}
