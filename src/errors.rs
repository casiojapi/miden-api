use std::{
    ffi,
    fmt::{self, Display},
    io,
};

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
    IOError(io::Error),
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

#[derive(Responder)]
pub enum ApiError {
    #[response(status = 500, content_type = "json")]
    Cli(String),
}

impl From<CliError> for ApiError {
    fn from(v: CliError) -> Self {
        Self::Cli(format!("{:?}", v))
    }
}
