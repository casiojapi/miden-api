#[macro_use]
extern crate rocket;

use rocket_cors::{AllowedOrigins, CorsOptions};
use errors::ApiError;
use rocket::serde::json::{json, Json, Value};
use rocket::serde::{Deserialize, Serialize};
use wrapper::CliWrapper;

mod errors;
mod helpers;
mod txinfo;
mod wrapper;

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

#[get("/<username>/info", rank = 2)]
fn get_user(username: &str) -> Result<Value, ApiError> {
    let client = CliWrapper::new(username.into());
    let account = client.create_account()?;
    let balance = client.get_account_balance()?;
    Ok(json!({"username": username, "balance": balance, "address": account}))
}

#[get("/<username>/faucet", rank = 2)]
async fn faucet_fund(username: &str) -> Result<String, ApiError> {

    println!("Requesting faucet for username: {}", username);
    let client = CliWrapper::from_username(username.into()).await?;
    println!("client");
    // let client = CliWrapper::new(username.into());
    client.init_user()?;
    let (note_id, _) = client.faucet_request(100).await?;

    println!("mainrs faucet request: {}", note_id);
    client.consume_and_sync(&note_id).await?;
    println!("consume and sync");
    Ok("funded".to_string())
}

#[get("/<username>/balance", rank = 2)]
async fn get_balance(username: &str) -> Result<String, ApiError> {
    let client = CliWrapper::from_username(username.into()).await?;
    client.init_user()?;
    let balance = client.get_account_balance()?;
    Ok(balance)
}

use txinfo::TxInfo;
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct TxTable {
    transactions: Vec<TxInfo>,
}

#[get("/<username>/transactions", rank = 2)]
async fn get_history(username: &str) -> Result<Json<TxTable>, ApiError> {
    let client = CliWrapper::from_username(username.into()).await?;
    client.init_user()?;
    let transactions = client.sql_get_transactions();
    Ok(Json(TxTable { transactions }))
}

#[get("/<username>/note/to/<to>/asset/<asset>", rank = 2)]
async fn send_note(username: &str, to: &str, asset: &str) -> Result<String, ApiError> {
    let sender = CliWrapper::from_username(username.into()).await?;
    let receiver = CliWrapper::from_username(to.into()).await?;
    let receiver_acc = receiver.get_default_account_or_err()?;
    let note_id = sender.create_note_and_sync(receiver_acc, asset.into()).await?;
    sender.export_note_to_path(&note_id, receiver.get_user_path())?;
    receiver.consume_and_sync(&note_id).await?;
    Ok(note_id)
}

#[get("/users", rank = 2)]
fn get_users() -> Result<Json<Users>, ApiError> {
    Ok(Json(Users { users: wrapper::list_users() }))
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct Users {
    users: Vec<String>,
}

#[launch]
fn run() -> _ {
    let cors = CorsOptions::default()
        .allowed_origins(AllowedOrigins::all())
        .allowed_methods(vec![rocket::http::Method::Get, rocket::http::Method::Post, rocket::http::Method::Options]
            .into_iter()
            .map(From::from)
            .collect())
        .allow_credentials(true);

    rocket::build()
        .mount("/", routes![ping])
        .mount(
            "/api/account",
            routes![
                new_user,
                get_user,
                send_note,
                faucet_fund,
                get_balance,
                get_history,
                get_users
            ],
        )
        .attach(cors.to_cors().unwrap())
}
