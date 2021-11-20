use std::process::exit;
use std::sync::Arc;
use clap::{App, Arg};
use cs140_common::padding::padding;
use cs140_network::ack::state_machine::{AckStateMachine, BYTE_IN_FRAME};

#[tokio::main]
async fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.format_timestamp_millis().init();
    let matches = App::new("Mac Perf Client")
        .version("1.0")
        .arg(Arg::new("connect")
            .short('c')
            .value_name("ADDRESS")
            .about("Connect to the target address")
            .required(true)
            .takes_value(true)).get_matches();
    if let Some(address) = matches.value_of("connect") {
        let address: u8 = address.parse().unwrap();
        let mut client = AckStateMachine::new(0, 0, 255);
        client.append(padding().take(1024 * 128));
        println!("Connecting to {}.", address);
        let mut receiver = client.size_channel();
        tokio::spawn(async move {
            let mut time = 0;
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                time += 1;
                let new_size = receiver.recv().await.unwrap();
                println!("{:.3} Kbps", (new_size as f64) * (BYTE_IN_FRAME as f64) / time as f64 / 1000.0 * 8.0);
            }
        });
        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.unwrap();
            exit(0);
        });
        client.work().await;
    }
}