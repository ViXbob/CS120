use std::process::exit;
use clap::{App, Arg};
use cs140_common::padding;
use cs140_network::ack::ack::AckPackage;
use cs140_network::ack::state_machine::{AckStateMachine, BYTE_IN_FRAME, CONTENT_IN_FRAME};
use cs140_network::encoding::HandlePackage;

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
        /// server.work().await;
        let mut layer = server.ack_layer;
        let mut total_ack_count = 0;
        let mut total_time = 0;
        loop {
            let ack_count = async {
                let mut ack = 0;
                let mut total_time = std::time::Duration::from_secs(1);
                loop {
                    let start = std::time::Instant::now();
                    let package = tokio::time::timeout(total_time, layer.receive()).await;
                    match package {
                        Ok(package) => {
                            ack += 1;
                            layer.send(AckPackage::new(padding::padding().take(CONTENT_IN_FRAME), 0, 0,false, true, 0, 0)).await;
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
            if total_ack_count > 0 {
                println!("{:.3} Kbps", (total_ack_count * BYTE_IN_FRAME) as f32 / total_time as f32 / 1000.0 * 8.0);
            }
        }
    }
}