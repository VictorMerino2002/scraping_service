#[derive(Debug, Clone)]
pub enum Error {
    NotFound(String),
    InvalidInput(String),
    DatabaseError(String),
    NetworkError(String),
    Unauthorized(String),
    Unknown(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotFound(message)
            | Error::InvalidInput(message)
            | Error::DatabaseError(message)
            | Error::NetworkError(message)
            | Error::Unauthorized(message)
            | Error::Unknown(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for Error {}
