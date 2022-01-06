use std::net::Ipv4Addr;
use cs140_util::redirect::run_unix_redirect_server;

#[tokio::main]
async fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.format_timestamp_millis().init();
    run_unix_redirect_server(Ipv4Addr::new(10, 19, 75, 4), Ipv4Addr::new(10, 19, 73, 32)).await;
}