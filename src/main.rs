use std::process::Command; 
use std::path::Path;

fn main() {
    pwd();
//    test_miden();
//    exist_directory("/miden/noexiste".to_string());
//    init_user("joel".to_string());
    create_wallet("joel".to_string())
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

fn exist_directory(usr_id: String) -> bool {
    println!("{}", Path::new("/miden/hosts").exists());
    return Path::new("/miden/hosts").exists()
}

fn init_user(usr: String){
    let path = format!("/tmp/{}",usr);
    let output = Command::new("bash").arg("-c").arg(format!("mkdir {} ; cd {} ;  miden init --rpc 18.203.155.106",path,path))
        .output().expect("No se uqe hace esto");
    println!("Inicializo miden")
}

fn create_wallet(usr: String){
    let path = format!("/tmp/{}",usr);
    let output = Command::new("bash").arg("-c").arg(format!("cd {} ;  miden new-wallet -m",path))
        .output().expect("No se uqe hace esto");
    println!("status: {}", output.status);
    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
}
