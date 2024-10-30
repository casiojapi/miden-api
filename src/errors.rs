use std::{ffi, fmt::{self}, io};

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
    CreateNote,
    ExportNote,
    NoDefaultAccount,
    PollTimeoutError,
    AccountBalance,
    NoAccounts,
    ListNotes,
    BadUsername(ffi::OsString),
    Regex(regex::Error),
    ReqwestError(reqwest::Error),
    IOError(io::Error)
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl From<reqwest::Error> for CliError {
    fn from(value: reqwest::Error) -> Self {
        CliError::ReqwestError(value)
    }
}

impl From<io::Error> for CliError {
    fn from(value: io::Error) -> Self {
        CliError::IOError(value)
    }
}

impl From<ffi::OsString> for CliError {
    fn from(value: ffi::OsString) -> Self {
        CliError::BadUsername(value)
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
