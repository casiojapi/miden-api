use std::process::Command; 

fn main() {
    pwd();
    test_miden();
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

