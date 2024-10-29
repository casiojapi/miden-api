#[macro_use]
extern crate rocket;

use errors::ApiErrorResponder;
use wrapper::CliWrapper;

mod errors;
mod helpers;
mod other;
mod wrapper;

#[get("/ping")]
fn ping() -> &'static str {
    "pong"
}

#[get("/<id>/new", rank = 2)]
fn new(id: &str) -> Result<String, ApiErrorResponder> {
    let client = CliWrapper::new(id.into());
    client.init_user()?;
    client.create_account()?;
    Ok("".into())
}

#[get("/<id>/note/from/<from>/to/<to>/asset/<asset>", rank = 2)]
fn send_note(id: &str, from: &str, to: &str, asset: &str) {
    let client = CliWrapper::new(id.into());
    let note_id = client.create_note("".into(), "".into());

}

#[get("/<id>/note/receive/<note_id>", rank = 2)]
fn receive_note(id: &str, note_id: &str) {
}

#[launch]
fn run() -> _ {
    rocket::build()
        .mount("/", routes![ping])
        .mount("/account", routes![new])
}
