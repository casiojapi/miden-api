use std::fmt::{self};

use rocket::http::Status;

#[derive(Debug)]
pub enum CliError {
    CreateUserDir,
    MidenInit,
    CreateAccount,
    ConsumeNote,
    ImportNote,
    PathNotFound,
    SyncError,
    ParseError,
    CreateNote
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

pub enum ApiError {
    Cli(CliError),
}

impl From<CliError> for ApiError {
    fn from(v: CliError) -> Self {
        Self::Cli(v)
    }
}

pub type ApiErrorResponder = (Status, String);

impl From<CliError> for ApiErrorResponder {
    fn from(value: CliError) -> Self {
        (Status::InternalServerError, value.to_string())
    }
}

impl From<ApiError> for ApiErrorResponder {
    fn from(value: ApiError) -> Self {
        match value {
            ApiError::Cli(we) => (Status::InternalServerError, we.to_string()),
        }
    }
}
