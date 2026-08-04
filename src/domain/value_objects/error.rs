#[derive(Debug, Clone)]
pub enum Error {
    NotFound(String),
    InvalidInput(String),
    DatabaseError(String),
    NetworkError(String),
    Unauthorized(String),
    Unknown(String),
}
