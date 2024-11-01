use sqlite;
use std::path::Path;
use rocket::serde:: Serialize;

#[derive(Debug)]
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct TxInfo{
    note_id :String,
    acc_sender :String,
    acc_recipient :String,
    acc_recipient_user_id :String,
    faucet :String,
    value :String,
    timestamp: String,
    transaction_type: String,
}

impl TxInfo {
    pub fn from_row(row: &[(&str, Option<&str>)] ) -> Self {
        return Self{
         note_id : row[0].1.unwrap().to_string(),
         acc_sender : row[1].1.unwrap().to_string(),
         acc_recipient : row[2].1.unwrap().to_string(),
         acc_recipient_user_id : row[3].1.unwrap().to_string(),
         faucet : row[4].1.unwrap().to_string(),
         value : row[5].1.unwrap().to_string(),
         timestamp : row[6].1.unwrap().to_string(),
         transaction_type : row[7].1.unwrap().to_string(),
        };
    }

    pub fn from_values(note_id: String,
        acc_sender:String,
        acc_recipient:String,
        acc_recipient_user_id:String,
        faucet:String,
        value:String,
        timestamp:String,
        transaction_type:String
        ) -> Self {
        return Self{
            note_id,
            acc_sender,
            acc_recipient,
            acc_recipient_user_id,
            faucet,
            value,
            timestamp,
            transaction_type,
        }
    }

    pub fn to_database(&self, user_db_path:String){
        let path = Path::new(&user_db_path);
        let conection = sqlite::open(path).unwrap();
        let tmp = r#"INSERT INTO "tx_extension_table" VALUES "#;
        let data = (&self.note_id,
            &self.acc_sender,
            &self.acc_recipient,
            &self.acc_recipient_user_id,
            &self.faucet,
            &self.value,
            &self.timestamp,
            &self.transaction_type
            );
        let query_insert = format!("{} {:?}",tmp, data);
        let _ = conection.execute(&query_insert);
    }

}

pub fn init_tx_table(user_db_path:String) {
    let path = Path::new(&user_db_path);
    let conection = sqlite::open(path).unwrap();
    let query_create = r#"CREATE TABLE "tx_extension_table" (
        "note_id" TEXT,
        "acc_sender" TEXT,
        "acc_recipient" TEXT,
        "acc_recipient_user_id" TEXT,
        "faucet" TEXT,
        "value" TEXT,
        "timestamp" TEXT,
        "transaction_type" TEXT,
        PRIMARY KEY("note_id")
        );"# ;
    let _ = conection.execute(query_create);
}


pub fn get_tx_data(user_db_path:String) -> Vec<TxInfo> {
    let path = Path::new(&user_db_path);
    let conection = sqlite::open(path).unwrap();
    let mut data =Vec::new();
    let query = "SELECT * FROM tx_extension_table";
    let _ = conection.iterate(query,|row| {
        data.push(TxInfo::from_row(&row));
        true});
    return data

}
