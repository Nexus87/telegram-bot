extern crate futures;
extern crate telegram_bot;
extern crate tokio;

use std::env;

use futures::Stream;
use telegram_bot::*;

fn main() {

    let token = env::var("TELEGRAM_BOT_TOKEN").unwrap();
    let api = Api::configure(token).build().unwrap();

    // Convert stream to the stream with errors in result
    let stream = api.stream().then(|mb_update| {
        let res: Result<Result<Update, Error>, ()> = Ok(mb_update);
        res
    });

    // Print update or error for each update.
    let future = stream.for_each(|mb_update| {
        println!("{:?}", mb_update);
        Ok(())
    });

    tokio::run(future);
}
