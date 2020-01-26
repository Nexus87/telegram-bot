extern crate futures;
extern crate telegram_bot;
extern crate tokio;

use std::env;

use futures::StreamExt;
use telegram_bot::*;

#[tokio::main]
async fn main() -> Result<(), Error>{

    let token = env::var("TELEGRAM_BOT_TOKEN").unwrap();
    let api = Api::configure(token).build().unwrap();

    // Fetch new updates via long poll method
    let mut stream= api.stream();
    while let Some(update) = stream.next().await {
        let update = update?;
        // If the received update contains a new message...
        if let UpdateKind::Message(message) = update.kind {

            if let MessageKind::Text {ref data, ..} = message.kind {
                // Print received text message to stdout.
                println!("<{}>: {}", &message.from.first_name, data);

                // Answer message with "Hi".
                api.send(message.text_reply(
                    format!("Hi, {}! You just wrote '{}'", &message.from.first_name, data)
                )).await?;
            }
        }

    };
    Ok(())

}
