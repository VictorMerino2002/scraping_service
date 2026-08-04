use crate::domain::value_objects::{Error, JobAction};

pub trait ActionSerializer: Send + Sync {
    type Input;

    fn from(&self, input: Self::Input) -> Result<Vec<JobAction>, Error>;

    fn to(&self, actions: &[JobAction]) -> Result<Self::Input, Error>;
}
