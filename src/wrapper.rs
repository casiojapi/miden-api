//TODO: El crear note quedo medio raro (al final del create note mando la info a la db),
//de momento estamos usando el note_id como tx_id en la db
//hay que ver como conseguir el user_id, de momento estamos guardando 2 veces el addres del target
//
//Al final de create_wallet estamos corriendo sql_init_table, agrega la tabla a la base de datos
//
//Las filas de las db de las transacciones las trae como Vec<TxInfo>. Hay que decidir como
//serializarlas

use regex::Regex;
use rocket::tokio;
use rocket::tokio::time::sleep;
use sqlite;
use sqlite::Connection;

use crate::errors::{CmdError, WrapperError};
use crate::txinfo::TxInfo;

#[cfg(feature = "debug")]
use crate::stdpr;

pub(crate) const MIDEN_CLIENT_CLI_VAR: &'static str = "MIDEN_CLIENT_CLI";
pub(crate) const USERS_DB_DIR_VAR: &'static str = "USERS_DB_DIR";
pub(crate) const USERNAME_DB_DIR_VAR: &'static str = "USERNAME_DB_DIR";

pub const FAUCET: &str = "0xa0e61d8a3f8b50be";

use std::{
    env,
    ffi::OsStr,
    fs::{self, DirEntry},
    num::ParseIntError,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug)]
struct SyncStatus {
    block: usize,
    new_pub_notes: usize,
    notes_updated: usize,
    notes_consumed: usize,
    accounts_updated: usize,
    commited_transactions: usize,
}

impl SyncStatus {
    pub fn from_log(s: &str) -> Result<Self, ParseIntError> {
        let _re = r"State synced to block (\d+)\nNew public notes: (\d+)\nTracked notes updated: (\d)\nTracked notes consumed: (\d+)\nTracked accounts updated: (\d+)\nCommited transactions: (\d)";
        let re = Regex::new(_re).unwrap();
        let res = re.captures(s).unwrap();
        Ok(Self {
            block: res[1].to_string().parse::<usize>()?,
            new_pub_notes: res[2].to_string().parse::<usize>()?,
            notes_updated: res[3].to_string().parse::<usize>()?,
            notes_consumed: res[4].to_string().parse::<usize>()?,
            accounts_updated: res[5].to_string().parse::<usize>()?,
            commited_transactions: res[6].to_string().parse::<usize>()?,
        })
    }
}

#[derive(Debug)]
pub enum NoteStatus {
    Expected,
    Committed,
    Consumed,
}

pub type WResult<T> = Result<T, WrapperError>;
pub type CmdResult = Result<String, CmdError>;

pub struct CliWrapper {
    bin: String,
    user_id: String,
    username: String,
}

impl CliWrapper {
    pub fn new(user_id: String, username: String) -> Self {
        let bin = env::var(MIDEN_CLIENT_CLI_VAR).unwrap_or("/bin/miden".into());
        println!("bin: {:?}", bin);
        Self {
            bin,
            user_id,
            username,
        }
    }

    pub async fn from_username(username: String) -> WResult<Self> {
        let bin = env::var(MIDEN_CLIENT_CLI_VAR).unwrap_or("/bin/miden".into());
        let dir = format!("{}/{}", Self::username_db_dir(), username);

        let mut user_id_dir_data = tokio::fs::read_dir(dir).await?;
        let mut user_id_dir = Vec::new();
        loop {
            if let Some(next) = user_id_dir_data.next_entry().await? {
                user_id_dir.push(next);
            } else {
                break;
            }
        }
        let user_id_dir = user_id_dir.pop().ok_or(WrapperError::PathNotFound)?;

        let user_id: String = user_id_dir.file_name().into_string()?;

        Ok(Self {
            bin,
            user_id,
            username,
        })
    }

    pub fn get_account_balance(&self) -> WResult<String> {
        let account_id = self.get_default_account_or_err()?;
        let output = self._miden_show_account(account_id)?;
        let lines = output.lines().filter(|line| line.contains(FAUCET)).last();
        let binding = lines.unwrap_or("│ f ┆ f ┆ 0 │");
        let balance = binding
            .split("┆")
            .last()
            .unwrap()
            .replace(" ", "")
            .replace("│", "");
        Ok(balance.to_string())
    }

    fn username_db_dir() -> String {
        env::var(USERNAME_DB_DIR_VAR).unwrap_or("/tmp/usernames".into())
    }

    fn user_db_dir() -> String {
        env::var(USERS_DB_DIR_VAR).unwrap_or("/tmp/users".into())
    }

    fn get_username_map_path(&self) -> String {
        format!(
            "{}/{}/{}",
            Self::username_db_dir(),
            self.username,
            self.user_id
        )
    }

    pub fn get_user_path(&self) -> String {
        format!("{}/{}", Self::user_db_dir(), self.user_id)
    }

    fn get_user_db_path(&self) -> String {
        format!("{}/store.sqlite3", self.get_user_path())
    }

    fn sql_create_connection(&self) -> Connection {
        let path_db = self.get_user_db_path();
        let path_db = Path::new(&path_db);
        let connection = sqlite::open(path_db).unwrap();
        return connection;
    }

    fn sql_init_table(&self) -> () {
        let query_create = r#"CREATE TABLE "tx_extension_table" (
            "tx_id" TEXT,
            "acc_sender" TEXT,
            "acc_recipient" TEXT,
            "acc_recipient_user_id" TEXT,
            "faucet" TEXT,
            "value" TEXT,
            PRIMARY KEY("tx_id")
            );"#;
        let _ = self.sql_create_connection().execute(query_create);
        println!(
            "New tx_extension_table created in {}",
            self.get_user_db_path()
        )
    }

    fn sql_get_transactions(&self) -> String {
        let mut data = Vec::new();
        let conection = self.sql_create_connection();
        let query = "SELECT * FROM tx_extension_table";
        let _ = conection.iterate(query, |row| {
            data.push(TxInfo::from_row(&row).to_json());
            true
        });
        return format!("[{}]", data.join(","));
    }

    fn get_user_config_path(&self) -> String {
        format!("{}/{}", self.get_user_path(), "miden-client.toml")
    }

    fn _note_id_to_path(&self, note_id: &str) -> PathBuf {
        format!("{}/{}.mno", self.get_user_path(), note_id).into()
    }

    fn is_user_initialized(&self) -> bool {
        Path::new(&self.get_user_db_path()).exists()
    }

    fn create_user_dir(&self) -> WResult<()> {
        fs::create_dir_all(self.get_username_map_path())
            .map_err(|_| WrapperError::CreateUserDir)?;
        fs::create_dir_all(self.get_user_path()).map_err(|_| WrapperError::CreateUserDir)
    }

    fn _cd(&self) -> String {
        format!("cd {}", self.get_user_path())
    }

    fn _command_or_fail(&self, command: &mut Command, err: CmdError) -> CmdResult {
        let o = command.output()?;
        if !o.status.success() {
            error!("{}", String::from_utf8_lossy(&o.stderr));
            return Err(err);
        }
        Ok(String::from_utf8_lossy(&o.stdout).into_owned())
    }

    fn _miden_cmd<I, S>(&self, args: I, err: CmdError) -> CmdResult
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut cmd = Command::new(&self.bin);
        let r = cmd.current_dir(self.get_user_path()).args(args);
        self._command_or_fail(r, err)
    }

    fn _miden_init(&self) -> CmdResult {
        self._miden_cmd(["init", "--rpc", "18.203.155.106"], CmdError::MidenInit)
    }

    fn _miden_sync(&self) -> CmdResult {
        self._miden_cmd(["sync"], CmdError::MidenSyncError)
    }

    fn _miden_new_wallet_mut(&self) -> CmdResult {
        self._miden_cmd(["new-wallet", "-m"], CmdError::CreateAccount)
    }

    fn _miden_list_accounts(&self) -> CmdResult {
        self._miden_cmd(["account", "-l"], CmdError::ListAccounts)
    }

    fn _miden_show_account(&self, account_id: String) -> CmdResult {
        self._miden_cmd(["account", "--show", &account_id], CmdError::ShowAccount)
    }

    fn _miden_notes(&self) -> CmdResult {
        self._miden_cmd(["notes"], CmdError::ListNotes)
    }

    fn _miden_consume_notes(&self, account: String, notes: Vec<String>) -> CmdResult {
        let note_list_text = notes.join(" ");
        self._miden_cmd(
            ["consume-notes", "-a", &account, "-f", &note_list_text],
            CmdError::ConsumeNotes,
        )
    }

    fn _miden_consume_all_notes(&self, account: String) -> CmdResult {
        self._miden_cmd(
            ["consume-notes", "-a", &account, "-f"],
            CmdError::ConsumeNotes,
        )
    }

    fn _miden_import_notes(&self, notes: Vec<String>) -> CmdResult {
        let note_list_text = notes.join(" ");
        self._miden_cmd(["import", &note_list_text], CmdError::ImportNotes)
    }

    pub fn _miden_create_note(&self, target: String, amount: String) -> CmdResult {
        self._miden_cmd(
            [
                "send",
                "-t",
                &target,
                "-a",
                &format!("{}::{}", amount, FAUCET),
                "--note-type",
                "private",
                "--force",
            ],
            CmdError::CreateNote,
        )
    }

    pub fn _miden_export_note(&self, note_id: String) -> CmdResult {
        self._miden_cmd(
            [
                "export",
                "--note",
                "-e",
                "full",
                "-f",
                &format!("{}.mno", note_id),
                &note_id,
            ],
            CmdError::ExportNote,
        )
    }

    pub async fn faucet_request(&self, amount: usize) -> WResult<(String, PathBuf)> {
        let account_id = self
            .get_default_account()
            .ok_or(WrapperError::NoDefaultAccount)?;

        let body = format!(
            "{{ \"account_id\": \"{}\", \"is_private_note\": true, \"asset_amount\": {} }}",
            account_id, amount
        );

        let response = reqwest::Client::new()
            .post("https://testnet.miden.io/get_tokens")
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?;

        let note_id = response
            .headers()
            .get("note-id")
            .ok_or(WrapperError::ParseError)?
            .to_str()
            .map(|x| x.to_string())
            .map_err(|_| WrapperError::ParseError)?;

        let note = response.bytes().await?;

        let note_path: PathBuf = format!("{}/{}.mno", self.get_user_path(), note_id).into();
        tokio::fs::write(&note_path, note).await?;

        Ok((note_id, note_path))
    }

    fn sync(&self) -> WResult<SyncStatus> {
        let o = self._miden_sync()?;
        SyncStatus::from_log(&o).map_err(|_| WrapperError::CreateSyncStatus)
    }

    async fn poll_status_until_change(
        &self,
        curr_status: &SyncStatus,
        compare_with: &str,
        change_size: usize,
    ) -> WResult<()> {
        let mut counter = 0;
        loop {
            let status = self.sync()?;
            if counter % 4 == 0 {
                println!("scanning... {:?}", status);
            }
            let delta = match compare_with {
                "block" => status.block - curr_status.block,
                "new_pub_notes" => status.new_pub_notes - curr_status.new_pub_notes,
                "notes_updated" => status.notes_updated - curr_status.notes_updated,
                "notes_consumed" => status.notes_consumed - curr_status.notes_consumed,
                "accounts_updated" => status.accounts_updated - curr_status.accounts_updated,
                "commited_transactions" => {
                    status.commited_transactions - curr_status.commited_transactions
                }
                _ => panic!("please dont fuck up"),
            };
            if delta >= change_size || counter > 200 {
                println!("exiting... {:?}", status);
                break;
            }
            if counter > 200 {
                return Err(WrapperError::PollTimeoutError);
            }
            counter += 1;

            sleep(std::time::Duration::from_millis(100)).await;
        }
        Ok(())
    }

    pub fn init_user(&self) -> WResult<()> {
        if !self.is_user_initialized() {
            self.create_user_dir()?;
            self._miden_init()?;
        }
        println!("User initialized in {}", self.get_user_path());
        Ok(())
    }

    pub fn create_account(&self) -> WResult<String> {
        let output = self._miden_new_wallet_mut()?;
        let it: String = output
            .lines()
            .filter(|line| line.contains("To view account details execute"))
            .collect();
        let value = it.as_str().replace("`", "");
        let address: Option<String> = value
            .split(" ")
            .collect::<Vec<&str>>()
            .pop()
            .map(|x| x.to_string());
        println!(
            "New accoun {:?} created in {}",
            address.clone().unwrap(),
            self.get_user_db_path()
        );
        self.sql_init_table();
        address.ok_or(WrapperError::ParseError)
    }

    pub fn get_default_account(&self) -> Option<String> {
        let file_string = std::fs::read_to_string(self.get_user_config_path()).unwrap();
        let parsed_toml = file_string.parse::<toml::Table>().unwrap();
        let address = parsed_toml["default_account_id"]
            .as_str()
            .map(|x| x.to_string());
        //        println!("The default account in {} is {}",self.get_user_db_path(),&address.clone().unwrap());
        return address;
    }

    pub fn get_default_account_or_err(&self) -> WResult<String> {
        self.get_default_account()
            .ok_or(WrapperError::NoDefaultAccount)
    }

    pub fn get_list_accounts(&self) -> WResult<Vec<String>> {
        let output = self._miden_list_accounts()?;
        let filter = r"0x9[a-fA-F0-9]{15}";
        let regex = Regex::new(filter).unwrap();
        let account_ids: Vec<&str> = regex
            .find_iter(&output)
            .filter_map(|x| Some(x.as_str()))
            .collect();
        let account_ids: Vec<String> = account_ids.iter().map(|x| x.to_string()).collect();
        println!(
            "The accounts {:?} have been found in {}",
            account_ids,
            self.get_user_db_path()
        );
        Ok(account_ids)
    }

    pub fn create_note(&self, target: String, amount: String) -> WResult<String> {
        let output = self._miden_create_note(target.clone(), amount.clone())?;
        let note_id = output
            .split("Output notes:")
            .collect::<Vec<&str>>()
            .pop()
            .map(|x| x.replace(" ", "").replace("-", "").trim().to_string())
            .ok_or(WrapperError::ParseError)?;
        self.sync()?;
        let tx: TxInfo = TxInfo::from_values(
            note_id.clone(),
            self.get_default_account().unwrap(),
            target.clone(),
            target,
            FAUCET.to_string(),
            amount,
        );
        tx.to_database(self.get_user_db_path());
        return Ok(note_id);
    }

    pub fn export_note(&self, note_id: &str) -> WResult<Vec<u8>> {
        self._miden_export_note(note_id.to_string())?;
        let path = self._note_id_to_path(note_id);
        // TODO: use tokio::fs
        let bytes = std::fs::read(path).map_err(|_| WrapperError::PathNotFound)?;
        Ok(bytes)
    }

    pub fn export_note_to_path(&self, note_id: &str, path: String) -> WResult<()> {
        let bytes = self.export_note(note_id)?;
        // TODO: use tokio::fs
        std::fs::write(format!("{}/{}.mno", path, &note_id), bytes)
            .map_err(|_| WrapperError::PathNotFound)?;
        Ok(())
    }

    pub fn consume_notes(&self, account: String, notes: Vec<String>) -> WResult<()> {
        self._miden_consume_notes(account, notes)?;
        Ok(())
    }

    pub fn consume_all_notes(&self, account: String) -> WResult<()> {
        self._miden_consume_all_notes(account)?;
        Ok(())
    }

    pub fn import_note(&self, notes: Vec<PathBuf>) -> WResult<()> {
        let note_list_text: Vec<String> = notes
            .into_iter()
            .map(|p| {
                p.to_str()
                    .ok_or(WrapperError::PathNotFound)
                    .map(|x| x.to_string())
            })
            .collect::<WResult<Vec<String>>>()?;
        self._miden_import_notes(note_list_text)?;
        Ok(())
    }

    pub fn get_note(&self, note_id: &str) -> WResult<(NoteStatus, usize)> {
        let re = Regex::new(&format!(r"(?m)^ {} (\w+) .+ height (\d+)", note_id))
            .map_err(|e| WrapperError::Regex(e))?;
        let output = self._miden_notes()?;
        let capt = re.captures(&output).ok_or(WrapperError::ParseError)?;
        let status = match &capt[1] {
            "Expected" => NoteStatus::Expected,
            "Committed" => NoteStatus::Committed,
            "Consumed" => NoteStatus::Consumed,
            _ => panic!(),
        };

        let height = capt[2]
            .to_string()
            .parse::<usize>()
            .map_err(|_| WrapperError::ParseError)?;

        Ok((status, height))
    }

    pub async fn consume_and_sync(&self, note: &str) -> WResult<()> {
        // let note_paths: Vec<PathBuf> = notes.iter().map(|n| self._note_id_to_path(n)).collect();
        let note_path: PathBuf = self._note_id_to_path(&note);
        let status = self.sync()?;
        let account = self
            .get_default_account()
            .ok_or(WrapperError::NoDefaultAccount)?;
        self.import_note(vec![note_path])?;
        let (note_status, height) = self.get_note(&note)?;
        println!("Notestatus {:?} {}", note_status, height);
        match note_status {
            NoteStatus::Consumed => Ok(()),
            NoteStatus::Committed => self.consume_all_notes(account),
            NoteStatus::Expected => {
                self.poll_status_until_change(&status, "block", height - status.block)
                    .await?;
                self.consume_all_notes(account)?;
                self.poll_status_until_change(&status, "commited_transactions", 1)
                    .await?;
                Ok(())
            }
        }
    }

    pub async fn create_note_and_sync(&self, target: String, amount: String) -> WResult<String> {
        let status = self.sync()?;
        let note_id = self.create_note(target, amount)?;
        self.poll_status_until_change(&status, "commited_transactions", 1)
            .await;
        Ok(note_id)
    }
}

#[cfg(test)]
mod test {
    use rocket::tokio;

    use super::*;

    //    #[test]
    fn test_init() {
        env::set_var(USERS_DB_DIR_VAR, "/tmp/users_test");
        env::set_var(USERNAME_DB_DIR_VAR, "/tmp/usernames_test");
        env::set_var(MIDEN_CLIENT_CLI_VAR, "miden");
        let client_fran = CliWrapper::new("fran_id".into(), "fran".into());
        assert!(client_fran.init_user().is_ok());
        assert!(Path::new(&client_fran.get_user_config_path()).exists());
        assert!(Path::new(&client_fran.get_username_map_path()).exists());
    }

    // #[test]
    async fn test_from_username() {
        env::set_var(USERS_DB_DIR_VAR, "/tmp/users_test");
        env::set_var(USERNAME_DB_DIR_VAR, "/tmp/usernames_test");
        env::set_var(MIDEN_CLIENT_CLI_VAR, "miden");
        let _client_fran = CliWrapper::new("fran_id".into(), "fran".into());
        assert!(_client_fran.init_user().is_ok());

        let client_fran = CliWrapper::from_username("fran".into()).await;
        assert!(client_fran.is_ok());
        if let Ok(c) = client_fran {
            assert_eq!(c.username, "fran");
            assert_eq!(c.user_id, "fran_id");
        }
    }

    fn test() {
        env::set_var(USERS_DB_DIR_VAR, "/tmp/users_test");
        env::set_var(MIDEN_CLIENT_CLI_VAR, "miden");
        let client_fran = CliWrapper::new("fran_id".into(), "fran".into());
        client_fran.init_user();
        let client_joel = CliWrapper::new("joel_id".into(), "joel".into());
        client_joel.init_user();
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

    // #[test]
    //    fn test_create_note() {
    //        env::set_var(USERS_DB_DIR_VAR, "/tmp/users_test");
    //        env::set_var(MIDEN_CLIENT_CLI_VAR, "miden");
    //        let client_fran = CliWrapper::new("fran_id".into(), "fran".into());
    //        // client_fran.init_user();
    //        // client_fran.create_account();
    //
    //        let status = client_fran.sync().unwrap();
    //        println!("initial {:?}", status);
    //
    //        let (note_id, _) = client_fran.faucet_request(100).unwrap();
    //        println!("{}", note_id);
    //
    //        // client_fran.import_note(vec![note_path]);
    //        // client_fran.consume_all_notes(client_fran.get_default_account().unwrap());
    //        let o = tokio::runtime::Builder::new_multi_thread()
    //            .enable_all()
    //            .build()
    //            .unwrap()
    //            .block_on(async {
    //                client_fran.consume_and_sync(note_id).await;
    //
    //                // client_fran
    //                //     // .poll_status_until_change(status, "notes_consumed", 1)
    //                //     .poll_status_until_change(status, "block", 10)
    //                //     .await;
    //            });
    //        // println!("{}", o);
    //    }

    // #[test]
    fn test_get_note() {
        env::set_var(USERS_DB_DIR_VAR, "/tmp/users_test");
        env::set_var(MIDEN_CLIENT_CLI_VAR, "miden");
        let client_fran = CliWrapper::new("fran_id".into(), "fran".into());
        let note_info = client_fran
            .get_note("0x6227b0cddce9e35b9e886e8ba3498d150934721dfffbad075cc51de48247d38b");
        println!("{:?}", note_info);
    }

    #[tokio::test]
    async fn test_get_accounts() {
        env::set_var(USERS_DB_DIR_VAR, "/tmp/users_test");
        env::set_var(USERNAME_DB_DIR_VAR, "/tmp/usernames");
        env::set_var(MIDEN_CLIENT_CLI_VAR, "miden");
        let _client_fran = CliWrapper::new("fran_id".into(), "fran".into());
        let _ = _client_fran.init_user();
        let _ = _client_fran.create_account();
        //
        let (note_id, _) = _client_fran.faucet_request(100).await.unwrap();
        _client_fran.consume_and_sync(&note_id).await.unwrap();
        let res = _client_fran.get_list_accounts();

        let _client_joel = CliWrapper::new("joel_id".into(), "joel".into());
        _client_joel.init_user();
        let _ = _client_joel.create_account();
        let target = _client_joel.get_default_account().unwrap();
        _client_fran.create_note(target.clone(), "9".to_string());

        _client_fran.create_note(target, "1".to_string());

        let balance = _client_fran.get_account_balance().unwrap();
        let data = _client_fran.sql_get_transactions();
        let d = format!("{:?}", data);
        assert_eq!(d, "2 transacciones")
    }
}
