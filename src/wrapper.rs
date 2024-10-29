use crate::errors::Error;

#[cfg(debug)]
use crate::stdpr;

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

pub type WResult<T> = Result<T, Error>;

pub struct CliWrapper {
    bin: String,
    user_id: String,
}

impl CliWrapper {
    pub fn new(user_id: String) -> Self {
        let bin = env::var("MIDEN_CLIENT_CLI").unwrap_or("/bin/miden".into());
        Self { bin, user_id }
    }

    fn users_db_dir() -> String {
        env::var("USERS_DB_DIR").unwrap_or("/tmp/users".into())
    }

    fn get_user_path(&self) -> String {
        format!("{}/{}", Self::users_db_dir(), self.user_id)
    }

    fn get_user_db_path(&self) -> String {
        format!("{}/store.sqlite3", self.get_user_path())
    }

    fn is_user_initialized(&self) -> bool {
        Path::new(&self.get_user_db_path()).exists()
    }

    fn create_user_dir(&self) -> WResult<()> {
        fs::create_dir_all(self.get_user_path()).map_err(|_| Error::CreateUserDir)
    }

    fn _cd(&self) -> String {
        format!("cd {}", self.get_user_path())
    }

    fn _miden_init(&self) -> String {
        format!("{} && {} init --rpc 18.203.155.106", self._cd(), self.bin)
    }

    fn _miden_sync(&self) -> String {
        format!("{} && {} sync", self._cd(), self.bin)
    }

    fn _miden_new_wallet_mut(&self) -> String {
        format!("{} && {} new-wallet -m", self._cd(), self.bin)
    }

    fn _miden_consume_notes(&self, account: String, notes: Vec<String>) -> String {
        let note_list_text = notes.join(" ");
        let cmd = format!("consume-notes -a {} -f {}", account, note_list_text);
        format!("{} && {} {}", self._cd(), self.bin, cmd)
    }

    fn _miden_import_notes(&self, notes: Vec<String>) -> String {
        let note_list_text = notes.join(" ");
        let cmd = format!("import {}", note_list_text);
        format!("{} && {} {}", self._cd(), self.bin, cmd)
    }

    fn sync(&self) -> WResult<()> {
        Command::new("bash")
            .arg("-c")
            .arg(self._miden_sync())
            .output()
            .map_err(|_| Error::SyncError)?;
        Ok(())
    }

    pub fn init_user(&self) -> WResult<()> {
        if !self.is_user_initialized() {
            self.create_user_dir()?;
            Command::new("bash")
                .arg("-c")
                .arg(self._miden_init())
                .output()
                .map_err(|_| Error::MidenInit)?;
        }
        Ok(())
    }

    pub fn create_account(&self) -> WResult<()> {
        Command::new("bash")
            .arg("-c")
            .arg(self._miden_new_wallet_mut())
            .output()
            .map_err(|_| Error::CreateAccount)?;
        Ok(())
    }

    pub fn list_accounts(&self) {}

    pub fn create_note(&self) {
        // create_note
        // sync
    }

    pub fn export_note(&self) {}

    pub fn consume_notes(&self, account: String, notes: Vec<String>) -> WResult<()> {
        self.sync()?;
        Command::new("bash")
            .arg("-c")
            .arg(self._miden_consume_notes(account, notes))
            .output()
            .map_err(|_| Error::ConsumeNote)?;
        Ok(())
    }

    pub fn import_note(&self, notes: Vec<PathBuf>) -> WResult<()> {
        let note_list_text: Vec<String> = notes
            .into_iter()
            .map(|p| p.to_str().ok_or(Error::PathNotFound).map(|x| x.to_string()))
            .collect::<WResult<Vec<String>>>()?;

        Command::new("bash")
            .arg("-c")
            .arg(self._miden_import_notes(note_list_text))
            .output()
            .map_err(|_| Error::ImportNote)?;
        Ok(())
    }
}
