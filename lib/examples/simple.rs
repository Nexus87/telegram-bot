extern crate futures;
extern crate telegram_bot;
extern crate tokio;

use std::env;

use futures::Stream;
use telegram_bot::*;

fn main() {

    let token = env::var("TELEGRAM_BOT_TOKEN").unwrap();
    let api = Api::configure(token).build().unwrap();

    // Fetch new updates via long poll method
    let stream= api.stream()
        .map_err(|_| ())
        .for_each(move |update| {

        // If the received update contains a new message...
        if let UpdateKind::Message(message) = update.kind {

            if let MessageKind::Text {ref data, ..} = message.kind {
                // Print received text message to stdout.
                println!("<{}>: {}", &message.from.first_name, data);

                // Answer message with "Hi".
                api.spawn(message.text_reply(
                    format!("Hi, {}! You just wrote '{}'", &message.from.first_name, data)
                ));
            }
        }

        Ok(())
    });

    tokio::run(stream);
}
