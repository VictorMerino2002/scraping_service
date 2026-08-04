use crate::domain::{entities::Job, value_objects::Error};

pub trait JobSerializer {
    fn from(&self, raw: &str) -> Result<Job, Error>;
}
