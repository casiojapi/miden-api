use std::{
    ffi,
    fmt::{self, Display},
    io,
};

use rocket::http::Status;

#[derive(Debug)]
pub enum CmdError {
    MidenInit,
    MidenSyncError,
    CreateAccount,
    ListAccounts,
    ShowAccount,
    ListNotes,
    ConsumeNotes,
    ImportNotes,
    CreateNote,
    ExportNote,
    IOError(io::Error),
}

impl From<io::Error> for CmdError {
    fn from(value: io::Error) -> Self {
        Self::IOError(value)
    }
}

#[derive(Debug)]
pub enum WrapperError {
    CreateSyncStatus,
    CreateUserDir,
    PathNotFound,
    ParseError,
    NoDefaultAccount,
    PollTimeoutError,
    Cmd(CmdError),
    BadUsername(ffi::OsString),
    Regex(regex::Error),
    ReqwestError(reqwest::Error),
    IOError(io::Error),
}

impl From<CmdError> for WrapperError {
    fn from(value: CmdError) -> Self {
        Self::Cmd(value)
    }
}

impl fmt::Display for WrapperError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl From<reqwest::Error> for WrapperError {
    fn from(value: reqwest::Error) -> Self {
        WrapperError::ReqwestError(value)
    }
}

impl From<io::Error> for WrapperError {
    fn from(value: io::Error) -> Self {
        WrapperError::IOError(value)
    }
}

impl From<ffi::OsString> for WrapperError {
    fn from(value: ffi::OsString) -> Self {
        WrapperError::BadUsername(value)
    }
}

#[derive(Responder)]
pub enum ApiError {
    #[response(status = 500, content_type = "json")]
    Wrapper(String),
}

impl From<WrapperError> for ApiError {
    fn from(v: WrapperError) -> Self {
        Self::Wrapper(format!("{:?}", v))
    }
}
