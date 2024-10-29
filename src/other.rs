use std::{path::Path, process::Command};

use crate::wrapper::CliWrapper;

pub(crate) const DB_PATH: &str = "./db";

fn main() {
    let client = CliWrapper::new("test_id".into());
    client.init_user();
    // client.sync();
    // client.create_account();
    // client.import_note(vec![
    //     PathBuf::from("/home/odysseus/Others/miden/miden-client/0xa1eb48688924fac439263d9a459621185232dac3c3c6ea04441f5c40796d8098.mno")
    // ]);
    client.consume_notes("0x9297feb1c3a6fae7".into(), vec!["0xa1eb48688924fac439263d9a459621185232dac3c3c6ea04441f5c40796d8098".into()]);


    // pwd();
//    test_miden();
//    exist_directory("/miden/noexiste".to_string());
    init_user("joel".to_string());
    create_account("joel".to_string());
    let a = get_default_account("joel".to_string());
    println!("Account: {a}");
//    get_balance("joel".to_string());
}

fn test_miden() {
    let output = Command::new("bash").arg("-c").arg("cd ~/miden/miden-client/ ; miden account -l")
        .output().expect("No se uqe hace esto");
    println!("status: {}", output.status);
    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
}

fn pwd() {
    let mut pwd = Command::new("pwd");
    let result = pwd.output().expect("pwd");
    println!("{}", String::from_utf8_lossy(&result.stdout));
}

//TODO: Puede que esta funcion este de mas. se termina resolviendo en una linea.
fn exist_directory(path: &String) -> bool {
    return Path::new(&path).exists()
}

fn init_user(usr: String){
    let path = format!("{}/{}",DB_PATH,usr);
    if exist_directory(&path) {
    }
    else {
        let output = Command::new("bash").arg("-c").arg(format!("mkdir {} && cd {} &&  miden init --rpc 18.203.155.106",path,path))
            .output().expect("No se uqe hace esto");
        println!("Inicializo miden")
    }
}

fn create_account(usr: String){
    let path = format!("{}/{}",DB_PATH,usr);
    let output = Command::new("bash").arg("-c").arg(format!("cd {} &&  miden new-wallet -m",path))
        .output().expect("No se uqe hace esto");
    println!("status: {}", output.status);
    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
}


fn get_default_account(usr: String) -> String {
    let path = format!("{}/{}/miden-client.toml",DB_PATH,usr);
    let file_string = std::fs::read_to_string(path).unwrap();
    let parsed_toml = file_string.parse::<toml::Table>().unwrap();
    //TODO: Hay que manejar el error de que no exista el campo "defaul_account_id". Estoy haciendo
    //el ejemplo con alice, tiene un toml pero nunca creo una wallet.
    let address = parsed_toml["default_account_id"].to_string();
    println!("{}",address);
    return address
}


fn get_balance(usr: String) -> Option<String>{
    let path = format!("{}/{}",DB_PATH,usr);
    let address = get_default_account(usr);
    let output = Command::new("bash").arg("-c").arg(format!("cd {} ;  miden account --show {}",path,address))
        .output().expect("No se uqe hace esto");
    let result = String::from_utf8_lossy(&output.stdout).into_owned();
    let it: String = result.lines().filter(|line| line.contains("0xa0e61d8a3f8b50be")).collect();
    let value  = it.as_str().replace(" ","").replace("│","");
    let number:Option<String> = value.split("┆").collect::<Vec<&str>>().pop().map(|x| x.to_string());
    println!("{:?}",number);
    return number;
}


//fn transfer(sender: String, target: String, amount: String) {
//    create_note()
//    export_note()
//    send_note_to_target()
//    import_note()
//    consume_note()
//}

fn create_note(sender: String, target: String, amount: String) -> String {
    let path = format!("/tmp/{}",sender);
    let faucet = "0xa0e61d8a3f8b50be".to_string();
    let output = Command::new("bash").arg("-c").arg(format!("cd {} ;  miden send -s {} -t {} -a {}::{} --force",path,sender,target,faucet,amount))
        .output().expect("No se uqe hace esto");
    //TODO: hacer que devuelva el id de la nota
    //TODO: Exportar nota
    return "Id_note".to_string()
}

fn export_note(){

}
