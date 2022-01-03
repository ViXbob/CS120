use log::trace;
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.format_timestamp_millis().init();
    trace!("ok");
    let socket = UdpSocket::bind("10.19.73.32:18888").await.unwrap();
    trace!("ok");
    let mut buf = [0u8; 65536];
    loop {
        trace!("ok");
        let (len, addr) = socket.recv_from(&mut buf).await.unwrap();
        trace!("from {:?}", addr);
        let result = &buf.clone()[0..len];
        trace!("buf: {:?}", result);
        trace!("receive completed!");
        let string : &str = std::str::from_utf8(result).expect("couldn't convert");
        println!("{}", string);
    }

    std::thread::park();
}