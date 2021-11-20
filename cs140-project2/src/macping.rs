use std::process::exit;
use std::sync::Arc;
use std::time::Duration;
use clap::{App, Arg};
use tokio::time::error::Elapsed;
use cs140_common::padding::padding;
use cs140_network::ack::ack::{AckLayer, AckPackage};
use cs140_network::ack::state_machine::{AckStateMachine, BYTE_IN_FRAME, CONTENT_IN_FRAME};
use cs140_network::encoding::HandlePackage;

const ADDRESS:u8 = 255;

#[tokio::main]
async fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.format_timestamp_millis().init();
    let matches = App::new("Mac Ping Client")
        .version("1.0")
        .arg(Arg::new("connect")
            .short('c')
            .value_name("ADDRESS")
            .about("Connect to the target address")
            .required(true)
            .takes_value(true)).get_matches();
    if let Some(address) = matches.value_of("connect") {
        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.unwrap();
            exit(0);
        });
        let address: u8 = address.parse().unwrap();
        let mut client = AckStateMachine::new(0, 0, ADDRESS);
        let mut layer = client.ack_layer;
        loop {
            layer.flush();
            tokio::time::sleep(std::time::Duration::from_secs(2));
            let time = tokio::time::timeout(std::time::Duration::from_secs(2), async {
                let start = std::time::Instant::now();
                for _ in 0..30 {
                    layer.send(AckPackage::new(padding().take(CONTENT_IN_FRAME), CONTENT_IN_FRAME, 0, false, false, 255, 255)).await;
                };
                loop {
                    let package = layer.receive().await;
                    if package.address().0 != ADDRESS{
                        break;
                    }
                }
                start.elapsed()
            }).await;
            match time {
                Ok(time) => {
                    println!("from {}: time={}ms", address, time.as_millis());
                }
                Err(_) => {
                    println!("timeout");
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }
}