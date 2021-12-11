use log::trace;
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.format_timestamp_millis().init();
    trace!("ok");
    let socket = UdpSocket::bind("10.19.75.77:28888").await.unwrap();
    trace!("ok");
    let mut buf = [0u8; 65536];
    loop {
        trace!("ok");
        let len = socket.recv(&mut buf).await.unwrap();
        let result = &buf.clone()[0..len];
        trace!("buf: {:?}", result);
        trace!("receive completed!");
    }

    std::thread::park();
}