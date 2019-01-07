extern crate telegram_bot;
extern crate tokio;

use std::env;

use telegram_bot::{Api, GetMe};
use tokio::prelude::future::*;

fn main() {
    let token = env::var("TELEGRAM_BOT_TOKEN").unwrap();

    let api = Api::configure(token).build().unwrap();
    let future = api.send(GetMe).map_err(|_| ()).map(|_| ());

    println!("{:?}", tokio::run(future))
}
