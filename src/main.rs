#[macro_use]
extern crate rocket;

use errors::ApiError;
use rocket::serde::json::{json, Json, Value};
use rocket::serde::{Deserialize, Serialize};
use wrapper::CliWrapper;

mod errors;
mod helpers;
mod txinfo;
mod wrapper;

// 0.0.0.0:8000/ping
#[get("/ping")]
fn ping() -> &'static str {
    "pong"
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct UserCreate {
    username: String,
}

#[post("/create", format = "json", data = "<user>", rank = 2)]
fn new_user(user: Json<UserCreate>) -> Result<Value, ApiError> {
    let username = user.username.to_string();
    let client = CliWrapper::new(username.clone());
    client.init_user()?;
    let account = client.create_account()?;
    Ok(json!({"username": username, "balance": 0, "address": account}))
}

// 0.0.0.0:8000/api/acount/<username>/info
#[get("/<username>/info", rank = 2)]
fn get_user(username: &str) -> Result<Value, ApiError> {
    let client = CliWrapper::new(username.into());
    let account = client.create_account()?;
    let balance = client.get_account_balance()?;
    Ok(json!({"username": username, "balance": balance, "address": account}))
}


// 0.0.0.0:8000/api/acount/<username>/faucet
#[get("/<username>/faucet", rank = 2)]
async fn faucet_fund(username: &str) -> Result<String, ApiError> {
    let client = CliWrapper::from_username(username.into()).await?;
    client.init_user()?;
    let (note_id, _) = client.faucet_request(100).await?;
    client.consume_and_sync(&note_id).await?;
    Ok("funded".to_string())
}

// 0.0.0.0:8000/api/acount/<username>/balance
#[get("/<username>/balance", rank = 2)]
async fn get_balance(username: &str) -> Result<String, ApiError> {
    let client = CliWrapper::from_username(username.into()).await?;
    client.init_user()?;
    let balance = client.get_account_balance()?;
    Ok(balance)
}

// 0.0.0.0:8000/api/acount/<username>/transactions
#[get("/<username>/transactions", rank = 2)]
async fn get_history(username: &str) -> Result<String, ApiError> {
    let client = CliWrapper::from_username(username.into()).await?;
    client.init_user()?;
    let data = client.sql_get_transactions();
    Ok(data)
}

// 0.0.0.0:8000/api/acount/<sender>/note/to/<target>/asset/<amount>
#[get("/<username>/note/to/<to>/asset/<asset>", rank = 2)]
async fn send_note(username: &str, to: &str, asset: &str) -> Result<String, ApiError> {
    let sender = CliWrapper::from_username(username.into()).await?;
    let receiver = CliWrapper::from_username(to.into()).await?;
    let receiver_acc = receiver.get_default_account_or_err()?;
    let note_id = sender
        .create_note_and_sync(receiver_acc, asset.into())
        .await?;
    sender.export_note_to_path(&note_id, receiver.get_user_path())?;
    receiver.consume_and_sync(&note_id).await?;
    Ok(note_id)
}

//no implementada
#[get("/<username>/note/receive/<note_id>", rank = 2)]
fn receive_note(username: &str, note_id: &str) {}


#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct Users {
    users: Vec<String>
}

// 0.0.0.0:8000/api/acount/users
#[get("/users", rank = 2)]
fn get_users() -> Result<Json<Users>,ApiError> {
    Ok(Json( Users { users: wrapper::list_users() } ))
}

#[launch]
fn run() -> _ {
    rocket::build().mount("/", routes![ping]).mount(
        "/api/account",
        routes![
        new_user,
        get_user,
        send_note, faucet_fund, get_balance, get_history, get_users],
    )
}
