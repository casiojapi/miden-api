//TODO: El crear note quedo medio raro (al final del create note mando la info a la db),
//de momento estamos usando el note_id como tx_id en la db
//hay que ver como conseguir el user_id, de momento estamos guardando 2 veces el addres del target
//
//Al final de create_wallet estamos corriendo sql_init_table, agrega la tabla a la base de datos
//
//Las filas de las db de las transacciones las trae como Vec<TxInfo>. Hay que decidir como
//serializarlas
//TODO: El create_notes tiene que hacer un sync atras. Una vez que se hizo eso se guarda la tx

use chrono::Utc;
use regex::Regex;
use rocket::tokio;
use rocket::tokio::time::sleep;

use crate::errors::{CmdError, WrapperError};
use crate::txinfo;
use crate::txinfo::TxInfo;

#[cfg(feature = "debug")]

pub(crate) const MIDEN_CLIENT_CLI_VAR: &'static str = "MIDEN_CLIENT_CLI";
pub(crate) const USERS_DB_DIR_VAR: &'static str = "USERS_DB_DIR";
pub const DEFAULT_DB_PATH: &str = "/tmp/users";

pub const FAUCET: &str = "0xa0e61d8a3f8b50be";

use std::{
    env,
    ffi::OsStr,
    fs::{self},
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
    username: String,
}

pub fn list_users() -> Vec<String> {
    let path = env::var(USERS_DB_DIR_VAR).unwrap_or(DEFAULT_DB_PATH.into());
    let cmd = Command::new("bash")
        .current_dir(path)
        .args(["-c", "ls -d *"])
        .output()
        .unwrap();
    let output = String::from_utf8_lossy(&cmd.stdout).into_owned();
    let lines = output
        .lines()
        .map(|x| x.to_string())
        .collect::<Vec<String>>();
    lines
}

impl CliWrapper {
    pub fn new(username: String) -> Self {
        let bin = env::var(MIDEN_CLIENT_CLI_VAR).unwrap_or("miden".into());
        println!("bin: {:?}", bin);
        Self { bin, username }
    }

    pub async fn from_username(username: String) -> WResult<Self> {
        let bin = env::var(MIDEN_CLIENT_CLI_VAR).unwrap_or("miden".into());
        let dir = format!("{}/{}", Self::user_db_dir(), username);

        let mut user_id_dir_data = tokio::fs::read_dir(dir).await?;
        let mut user_id_dir = Vec::new();
        loop {
            if let Some(next) = user_id_dir_data.next_entry().await? {
                user_id_dir.push(next);
            } else {
                break;
            }
        }

        Ok(Self { bin, username })
    }

    pub fn get_account_balance(&self) -> WResult<String> {
        let account_id = self.get_default_account_or_err()?;
        let output = self._miden_show_account(account_id)?;
        println!("{:?}", "casa");
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

    fn user_db_dir() -> String {
        env::var(USERS_DB_DIR_VAR).unwrap_or(DEFAULT_DB_PATH.into())
    }

    pub fn get_user_path(&self) -> String {
        format!("{}/{}", Self::user_db_dir(), self.username)
    }

    fn get_user_db_path(&self) -> String {
        format!("{}/store.sqlite3", self.get_user_path())
    }

    fn sql_init_table(&self) -> () {
        txinfo::init_tx_table(self.get_user_db_path());
        println!(
            "New tx_extension_table created in {}",
            self.get_user_db_path()
        )
    }

    pub fn sql_get_transactions(&self) -> Vec<TxInfo> {
        let transactions = txinfo::get_tx_data(self.get_user_db_path());
        return transactions;
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

    fn _miden_show_note(&self, note_id: &str) -> CmdResult {
        self._miden_cmd(["notes", "--show", &note_id], CmdError::ListNotes)
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
        println!("faucet request account_id: {}", account_id);
        let body = format!(
            "{{ \"account_id\": \"{}\", \"is_private_note\": true, \"asset_amount\": {} }}",
            account_id, amount
        );

        println!("faucet request body: {}", body);
        let response = reqwest::Client::new()
            .post("https://testnet.miden.io/get_tokens")
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?;

        println!("faucet request response: {:?}", response);
        let note_id = response
            .headers()
            .get("note-id")
            .ok_or(WrapperError::ParseError)?
            .to_str()
            .map(|x| x.to_string())
            .map_err(|_| WrapperError::ParseError)?;
        println!("faucet request note_id: {:?}", note_id);

        let note = response.bytes().await?;

        println!("faucet request note: {:?}", note);

        let note_path: PathBuf = format!("{}/{}.mno", self.get_user_path(), note_id).into();
        tokio::fs::write(&note_path, note).await?;

        println!("faucet request note_path: {}", note_path.display());
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
        match self.get_default_account() {
            Some(address) => Ok(address),
            None => {
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
                    "New account {:?} created in {}",
                    address.clone().unwrap(),
                    self.get_user_db_path()
                );
                self.sql_init_table();
                address.ok_or(WrapperError::ParseError)
            }
        }
    }

    pub fn get_default_account(&self) -> Option<String> {
        let file_string = std::fs::read_to_string(self.get_user_config_path()).ok();
        let parsed_toml = file_string?.parse::<toml::Table>().ok();
        let address = parsed_toml?
            .get("default_account_id")?
            .as_str()
            .map(|x| x.to_string());
        //        println!("The default account in {} is {}",self.get_user_db_path(),&address.clone().unwrap());
        return address;
    }

    pub fn get_default_account_or_err(&self) -> WResult<String> {
        self.get_default_account()
            .ok_or(WrapperError::NoDefaultAccount)
    }

    //    pub fn get_list_accounts(&self) -> WResult<Vec<String>> {
    //        let output = self._miden_list_accounts()?;
    //        let filter = r"0x9[a-fA-F0-9]{15}";
    //        let regex = Regex::new(filter).unwrap();
    //        let account_ids: Vec<&str> = regex
    //            .find_iter(&output)
    //            .filter_map(|x| Some(x.as_str()))
    //            .collect();
    //        let account_ids: Vec<String> = account_ids.iter().map(|x| x.to_string()).collect();
    //        println!(
    //            "The accounts {:?} have been found in {}",
    //            account_ids,
    //            self.get_user_db_path()
    //        );
    //        Ok(account_ids)
    //    }

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
            Utc::now().timestamp().to_string(),
            "output".to_string(),
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

    pub fn consume_notes(&self, account: String, note_id: &str) -> WResult<()> {
        self._miden_consume_notes(account.clone(), vec![note_id.to_string()])?;
        let (sender, amount) = self.get_note_info(note_id)?;
        println!("{:?}", (&sender, &amount));
        let tx = TxInfo::from_values(
            note_id.to_owned(),
            sender,
            account.clone(),
            account.clone(),
            FAUCET.to_owned(),
            amount,
            Utc::now().timestamp().to_string(),
            "input".to_string(),
        );
        let _ = tx.to_database(self.get_user_db_path());
        Ok(())
    }

    //    pub fn consume_all_notes(&self, account: String) -> WResult<()> {
    //        self._miden_consume_all_notes(account)?;
    //        Ok(())
    //    }

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

    pub fn get_note_info(&self, note_id: &str) -> WResult<(String, String)> {
        let output = self._miden_show_note(&note_id)?;
        let sender_line = output
            .lines()
            .filter(|line| line.contains("Sender"))
            .last()
            .unwrap();
        let amount_line = output
            .lines()
            .filter(|line| line.contains("Fungible Asset"))
            .last();

        //Aca se buscan por 16 hex y no por 15 forzando a 0x9 al inicio porque el funding inicial tiene como sender a
        //0xa....
        let filter = r"0x[a-fA-F0-9]{16}";
        let regex = Regex::new(filter).unwrap();
        let sender = regex.find(sender_line).unwrap().as_str().to_owned();

        let amount = amount_line
            .unwrap()
            .split(FAUCET)
            .last()
            .unwrap()
            .replace(" ", "");
        Ok((sender, amount))
    }

    pub async fn consume_and_sync(&self, note_id: &str) -> WResult<()> {
        // let note_paths: Vec<PathBuf> = notes.iter().map(|n| self._note_id_to_path(n)).collect();
        let note_path: PathBuf = self._note_id_to_path(&note_id);
        let status = self.sync()?;
        let account = self
            .get_default_account()
            .ok_or(WrapperError::NoDefaultAccount)?;
        self.import_note(vec![note_path])?;
        let (note_status, height) = self.get_note(&note_id)?;
        println!("{:?}", (&note_status, &height));
        //        let (sender, amount) = self.get_note_info(note_id)?;
        //        println!("{:?}",(&sender, &amount));
        //        let tx = TxInfo::from_values(
        //            note_id.to_owned(),
        //            sender,
        //            account.clone(),
        //            account.clone(),
        //            FAUCET.to_owned(),
        //            amount,
        //            Utc::now().timestamp().to_string(),
        //            "input".to_string()
        //            );
        //        let _ = tx.to_database(self.get_user_db_path());

        //        println!("Notestatus {:?} {}", note_status, height);
        match &note_status {
            NoteStatus::Consumed => Ok(()),
            NoteStatus::Committed => self.consume_notes(account, note_id),
            NoteStatus::Expected => {
                self.poll_status_until_change(&status, "block", height - status.block)
                    .await?;
                self.consume_notes(account, note_id)?;
                self.poll_status_until_change(&status, "commited_transactions", 1)
                    .await?;
                Ok(())
            }
        }
    }

    pub async fn create_note_and_sync(&self, target: String, amount: String) -> WResult<String> {
        let status = self.sync()?;
        let note_id = self.create_note(target, amount)?;
        let _ = self
            .poll_status_until_change(&status, "commited_transactions", 1)
            .await;
        Ok(note_id)
    }
}

#[cfg(test)]
mod test {
    use rocket::tokio;

    use super::*;
    #[tokio::test]
    async fn test_list_users() {
        let fran = CliWrapper::new("fran".into());
        let _ = fran.init_user();
        let joel = CliWrapper::new("joel".into());
        let _ = joel.init_user();
        let casio = CliWrapper::new("casio".into());
        let _ = casio.init_user();
        let list = list_users();
        println!("{}", list);
        assert_eq!(format!("{:?}", list), r#"["casio","fran","joel"]"#)
    }
    #[tokio::test]
    async fn test_get_accounts() {
        //        env::set_var(USERS_DB_DIR_VAR, "/tmp/users");
        //        env::set_var(MIDEN_CLIENT_CLI_VAR, "miden");
        //
        let sender = CliWrapper::new("fran".to_string());
        let _ = sender.init_user();
        let _ = sender.create_account();

        let res = sender.get_default_account();
        println!("The defaul account of fran is {:?}", res);
        let balance = sender.get_account_balance().unwrap();
        println!("The balance of fran is {:?}", balance);

        println!("Fran is asking for 100");
        assert_eq!(format!("{:?}", list), r#"["casio","fran","joel"]"#)
    }
    #[tokio::test]
    async fn test_get_accounts() {
        let sender = CliWrapper::new("fran".into());
        let _ = sender.init_user();
        let _ = sender.create_account();
        let res = sender.get_default_account();
        println!("{:?}", res);
        let balance = sender.get_account_balance().unwrap();
        println!("{:?}", balance);
        let (note_id, _) = sender.faucet_request(100).await.unwrap();
        println!("Fran has the note for 100");
        let _ = sender.consume_and_sync(&note_id).await.unwrap();
        println!("Note consumed for 100");
        //
        let receiver = CliWrapper::new("joel".into());
        println!("Consumio los 100");
        //
        let receiver = CliWrapper::new("joel".into());
        let _ = receiver.init_user();
        let _ = receiver.create_account();
        let target = receiver.get_default_account().unwrap();
        println!("Creo  la cuenta {:?}", target);
        //
        let note_id = sender
            .create_note_and_sync(target, "9".to_string())
            .await
            .unwrap();
        let _ = sender.export_note_to_path(&note_id, receiver.get_user_path());
        let _ = receiver.consume_and_sync(&note_id).await;
        let balance = sender.get_account_balance().unwrap();
        let data = sender.sql_get_transactions();
        println!("{:?}", data);
        let data = receiver.sql_get_transactions();
        println!("{:?}", data);
        assert_eq!(balance, "91")
    }
}
