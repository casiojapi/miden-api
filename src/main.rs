#[macro_use]
extern crate rocket;

use errors::ApiError;
use wrapper::CliWrapper;


mod errors;
mod helpers;
mod wrapper;
mod txinfo;

#[get("/ping")]
fn ping() -> &'static str {
    "pong"
}

#[get("/<username>/new/<id>", rank = 2)]
async fn new(username: &str, id: &str) -> Result<String, ApiError> {
    let client = CliWrapper::new(id.into(), username.into());
    client.init_user()?;
    let account = client.create_account()?;
    //let (note_id, _) = client.faucet_request(100).await?;
    //client.consume_and_sync(&note_id).await?;
    Ok(account)
}

#[get("/<username>/faucet", rank = 2)]
async fn faucet_fund(username: &str) -> Result<String, ApiError> {
    let client = CliWrapper::from_username(username.into()).await?;
    client.init_user()?;
    let (note_id, _) = client.faucet_request(100).await?;
    client.consume_and_sync(&note_id).await?;
    Ok("funded".to_string())
}

#[get("/<username>/balance", rank = 2)]
async fn get_balance(username: &str) -> Result<String, ApiError> {
    let client = CliWrapper::from_username(username.into()).await?;
    client.init_user()?;
    let balance = client.get_account_balance()?;
    Ok(balance)
}

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

fn get_notes() {}

#[get("/<username>/note/receive/<note_id>", rank = 2)]
fn receive_note(username: &str, note_id: &str) {}

#[launch]
fn run() -> _ {
    rocket::build().mount("/", routes![ping]).mount(
        "/account",
        routes![new, send_note, faucet_fund, get_balance],
    )
}
