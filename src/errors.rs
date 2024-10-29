#[derive(Debug)]
pub enum Error {
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
