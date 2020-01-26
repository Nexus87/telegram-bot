extern crate telegram_bot;
extern crate tokio;

use std::env;

use telegram_bot::{Api, GetMe, Error};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let token = env::var("TELEGRAM_BOT_TOKEN").unwrap();

    let api = Api::configure(token).build().unwrap();
    let result = api.send(GetMe).await?;
    println!("{:?}", result);
    Ok(())
}
