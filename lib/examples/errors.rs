extern crate futures;
extern crate telegram_bot;
extern crate tokio;

use std::env;
use futures::{StreamExt};
use telegram_bot::*;

#[tokio::main]
async fn main() {

    let token = env::var("TELEGRAM_BOT_TOKEN").unwrap();
    let api = Api::configure(token).build().unwrap();
    // Convert stream to the stream with errors in result
    let mut stream = api.stream();

    // Print update or error for each update.
    while let Some(mb_update) = stream.next().await {
        println!("{:?}", mb_update);
    }
}
