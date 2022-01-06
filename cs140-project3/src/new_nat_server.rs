use std::net::Ipv4Addr;
use cs140_util::new_nat::run_nat_server;

#[tokio::main]
async fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.format_timestamp_millis().init();
    run_nat_server(Ipv4Addr::new(10, 19, 73, 32), Ipv4Addr::new(10, 19, 75, 4)).await;
    std::thread::park();
}