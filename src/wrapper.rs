use crate::errors::CliError;

#[cfg(feature = "debug")]
use crate::stdpr;

pub(crate) const MIDEN_CLIENT_CLI_VAR: &'static str = "MIDEN_CLIENT_CLI";
pub(crate) const USERS_DB_DIR_VAR: &'static str = "USERS_DB_DIR";

pub const FAUCET: &str = "0xa0e61d8a3f8b50be";

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

pub type WResult<T> = Result<T, CliError>;

pub struct CliWrapper {
    bin: String,
    user_id: String,
}

impl CliWrapper {
    pub fn new(user_id: String) -> Self {
        let bin = env::var(MIDEN_CLIENT_CLI_VAR).unwrap_or("/bin/miden".into());
        Self { bin, user_id }
    }

    fn users_db_dir() -> String {
        env::var(USERS_DB_DIR_VAR).unwrap_or("/tmp/users".into())
    }

    fn get_user_path(&self) -> String {
        format!("{}/{}", Self::users_db_dir(), self.user_id)
    }

    fn get_user_db_path(&self) -> String {
        format!("{}/store.sqlite3", self.get_user_path())
    }

    fn get_user_config_path(&self) -> String {
        format!("{}/{}", self.get_user_path(), "miden-client.toml")
    }
    fn is_user_initialized(&self) -> bool {
        Path::new(&self.get_user_db_path()).exists()
    }

    fn create_user_dir(&self) -> WResult<()> {
        fs::create_dir_all(self.get_user_path()).map_err(|_| CliError::CreateUserDir)
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

    pub fn _miden_create_note(&self, target: String, amount: String) -> String {
        let cmd = format!(
            "send -t {} -a {}::{}  --note-type private --force",
            target, amount, FAUCET
        );
        format!("{} && {} {}", self._cd(), self.bin, cmd)
    }

    fn faucet_request(&self, amount: usize) -> String {
        let account_id = "0x9b7d69ffed23456a"; // TODO: get default account from self.user_id
        let body = format!(
            "{{ \"account_id\": \"{}\", \"is_private_note\": true, \"asset_amount\": {} }}",
            account_id, amount
        );
        let response = reqwest::blocking::Client::new()
            .post("https://testnet.miden.io/get_tokens")
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .unwrap();

        let note_id = response
            .headers()
            .get("note-id")
            .unwrap()
            .to_str()
            .map(|x| x.to_string())
            .unwrap();
        let note = response.bytes().unwrap();
        std::fs::write(format!("{}.mno", note_id), note);

        note_id
    }

    fn sync(&self) -> WResult<()> {
        Command::new("bash")
            .arg("-c")
            .arg(self._miden_sync())
            .output()
            .map_err(|_| CliError::SyncError)?;
        Ok(())
    }

    pub fn init_user(&self) -> WResult<()> {
        if !self.is_user_initialized() {
            self.create_user_dir()?;
            Command::new("bash")
                .arg("-c")
                .arg(self._miden_init())
                .output()
                .map_err(|_| CliError::MidenInit)?;
        }
        Ok(())
    }

    pub fn create_account(&self) -> WResult<String> {
        let output = Command::new("bash")
            .arg("-c")
            .arg(self._miden_new_wallet_mut())
            .output()
            .map_err(|_| CliError::CreateAccount)?;

        let result = String::from_utf8_lossy(&output.stdout).into_owned();
        let it: String = result
            .lines()
            .filter(|line| line.contains("To view account details execute"))
            .collect();
        let value = it.as_str().replace("`", "");
        let address: Option<String> = value
            .split(" ")
            .collect::<Vec<&str>>()
            .pop()
            .map(|x| x.to_string());
        address.ok_or(CliError::ParseError)
    }

    fn get_default_account(&self) -> Option<String> {
        //TODO armar el get_usr_config
        let file_string = std::fs::read_to_string(self.get_user_config_path()).unwrap();
        let parsed_toml = file_string.parse::<toml::Table>().unwrap();
        let address = parsed_toml["default_account_id"]
            .as_str()
            .map(|x| x.to_string());
        println!("{:?}", address);
        return address;
    }

    pub fn list_accounts(&self) {}

    pub fn create_note(&self, target: String, amount: String) -> Option<String> {
        let output = Command::new("bash")
            .arg("-c")
            .arg(self._miden_create_note(target, amount))
            .output()
            .map_err(|_| CliError::CreateAccount);
        let result = String::from_utf8_lossy(&output.unwrap().stdout).into_owned();
        println!("{:?}", result);
        let note_id: Option<String> = result
            .split("Output notes:")
            .collect::<Vec<&str>>()
            .pop()
            .map(|x| x.to_string())
            .map(|x| x.replace(" ", "").replace("-", ""));
        return note_id;
    }

    pub fn export_note(&self) {}

    pub fn consume_notes(&self, account: String, notes: Vec<String>) -> WResult<()> {
        self.sync()?;
        Command::new("bash")
            .arg("-c")
            .arg(self._miden_consume_notes(account, notes))
            .output()
            .map_err(|_| CliError::ConsumeNote)?;
        Ok(())
    }

    pub fn import_note(&self, notes: Vec<PathBuf>) -> WResult<()> {
        let note_list_text: Vec<String> = notes
            .into_iter()
            .map(|p| {
                p.to_str()
                    .ok_or(CliError::PathNotFound)
                    .map(|x| x.to_string())
            })
            .collect::<WResult<Vec<String>>>()?;

        Command::new("bash")
            .arg("-c")
            .arg(self._miden_import_notes(note_list_text))
            .output()
            .map_err(|_| CliError::ImportNote)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        env::set_var(USERS_DB_DIR_VAR, "/home/alba/miden/wraper-cli/tests/db");
        env::set_var(MIDEN_CLIENT_CLI_VAR, "miden");
        let client_fran = CliWrapper::new("fran".into());
        let client_joel = CliWrapper::new("joel".into());
        let target = client_joel.get_default_account();
        let id_note = client_fran
            .create_note(target.unwrap(), "1".to_string())
            .unwrap();
        assert_eq!(id_note, "asd");
        //
        //
        // do stuff

        client_fran.faucet_request(100);
    }
}
