use std::process::exit;
use std::sync::Arc;
use clap::{App, Arg};
use tokio::time::error::Elapsed;
use cs140_common::padding::padding;
use cs140_network::ack::ack::AckPackage;
use cs140_network::ack::state_machine::{AckStateMachine, BYTE_IN_FRAME, CONTENT_IN_FRAME};
use cs140_network::encoding::HandlePackage;

const ADDRESS: u8 = 255;

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
        let mut client = AckStateMachine::new(0, 0, ADDRESS);
        let mut layer = client.ack_layer;
        let mut total_ack_count = 0;
        let mut total_time = 0;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(2));
            let ack_count = async {
                let mut ack = 0;
                for _ in 0..48 {
                    layer.send(AckPackage::new(padding().take(CONTENT_IN_FRAME), CONTENT_IN_FRAME, 0, false, false, 255, 255)).await;
                };
                let mut total_time = std::time::Duration::from_secs(1);
                loop {
                    let start = std::time::Instant::now();
                    let package = tokio::time::timeout(total_time, layer.receive()).await;
                    match package {
                        Ok(package) => {
                            if package.has_ack() && package.address().0 != ADDRESS {
                                ack += 1;
                            }
                        }
                        Err(_) => {
                            break;
                        }
                    }
                    let time_cost = start.elapsed();
                    if total_time < time_cost {
                        break;
                    } else {
                        total_time -= time_cost;
                    }
                }
                ack
            }.await;
            total_ack_count += ack_count;
            total_time += 1;
            println!("{:.3} Kbps", (total_ack_count * BYTE_IN_FRAME) as f32 / total_time as f32 / 1000.0 * 8.0);
        }
    }
}