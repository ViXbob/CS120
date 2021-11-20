use std::process::exit;
use clap::{App, Arg};
use cs140_network::ack::state_machine::AckStateMachine;

#[tokio::main]
async fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.format_timestamp_millis().init();
    let matches = App::new("Mac Perf Server")
        .version("1.0")
        .arg(Arg::new("listen")
            .short('l')
            .value_name("ADDRESS")
            .about("Sets the address for the server")
            .required(true)
            .takes_value(true)).get_matches();
    if let Some(c) = matches.value_of("listen") {
        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.unwrap();
            exit(0);
        });
        let address = c.parse().unwrap();
        println!("Listening at {}.",address);
        let mut server = AckStateMachine::new(0, 0, address);
        server.work().await;
    }
}