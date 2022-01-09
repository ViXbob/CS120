use std::net::Ipv4Addr;
use cs140_util::redirect::run_unix_redirect_server;

fn read(buf: &mut String) {
    buf.clear();
    std::io::stdin().read_line(buf);
    let tmp = buf.trim().clone();
    *buf = String::from(tmp);
}

#[tokio::main]
async fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.format_timestamp_millis().init();
    println!("please type the local address, ");
    let mut buf: String = String::new();
    read(&mut buf);
    let local_addr: Ipv4Addr = buf.parse().unwrap();
    println!("please type the remote address, ");
    let mut buf: String = String::new();
    read(&mut buf);
    let remote_addr: Ipv4Addr = buf.parse().unwrap();
    // run_unix_redirect_server(Ipv4Addr::new(10, 19, 75, 4), Ipv4Addr::new(10, 19, 73, 32)).await;
    run_unix_redirect_server(local_addr, remote_addr).await;
}