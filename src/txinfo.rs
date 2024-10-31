use sqlite;
use std::path::Path;

#[derive(Debug)]
pub struct TxInfo{
    tx_id :String,
    acc_sender :String,
    acc_recipient :String,
    acc_recipient_user_id :String,
    faucet :String,
    value :String,
}

impl TxInfo {
    pub fn from_row(row: &[(&str, Option<&str>)] ) -> Self {
        return Self{
         tx_id : row[0].1.unwrap().to_string(),
         acc_sender : row[1].1.unwrap().to_string(),
         acc_recipient : row[2].1.unwrap().to_string(),
         acc_recipient_user_id : row[3].1.unwrap().to_string(),
         faucet : row[4].1.unwrap().to_string(),
         value : row[5].1.unwrap().to_string(),
        };
    }

    pub fn from_values(tx_id: String,
        acc_sender:String,
        acc_recipient:String,
        acc_recipient_user_id:String,
        faucet:String,
        value:String) -> Self {
        return Self{
            tx_id,
            acc_sender,
            acc_recipient,
            acc_recipient_user_id,
            faucet,
            value,
        }
    }

    pub fn to_database(&self, user_db_path:String){
        let path = Path::new(&user_db_path);
        let conection = sqlite::open(path).unwrap();
        let tmp = r#"INSERT INTO "tx_extension_table" VALUES "#;
        let data = (&self.tx_id,
            &self.acc_sender,
            &self.acc_recipient,
            &self.acc_recipient_user_id,
            &self.faucet,
            &self.value);
        let query_insert = format!("{} {:?}",tmp, data);
        let _ = conection.execute(&query_insert);
    }

    pub fn to_json(&self) -> String {
        return format!(r#"{{"tx_id" : "{}" , "acc_sender" : "{}" , "acc_recipient": "{}" , "acc_recipient_user_id" : "{}" , "faucet" : "{}" , "value" : "{}"}}"#,
            &self.tx_id,
            &self.acc_sender,
            &self.acc_recipient,
            &self.acc_recipient_user_id,
            &self.faucet,
            &self.value);
    }

}
