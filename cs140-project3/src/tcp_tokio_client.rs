use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use cs140_util::file_io;

fn read(buf: &mut String) {
    buf.clear();
    std::io::stdin().read_line(buf);
    let tmp = buf.trim().clone();
    *buf = String::from(tmp);
}

#[tokio::main]
async fn main() {
    let mut data: Vec<u8> = Vec::new();
    println!("please type the listening address,");
    let mut buf: String = String::new();
    read(&mut buf);
    let addr = buf.parse::<SocketAddr>().unwrap();
    let mut tcp = TcpListener::bind(addr).await.unwrap();
    let mut buf = [0u8; 6000];
    loop {
        let (mut socket, _) = tcp.accept().await.unwrap();
        let mut TIME = tokio::time::Instant::now();
        loop {
            let result = tokio::time::timeout(std::time::Duration::from_millis(2000),socket.read(&mut buf)).await;
            if result.is_err() {
                break;
            }
            let len = result.unwrap().unwrap();
            data.extend(buf[0 .. len].iter());
        }
        println!("time: {} ms", TIME.elapsed().as_millis());
        println!(
            "recv data: {:?}",
            std::str::from_utf8(data.as_ref()).unwrap_or("(invalid utf8)")
        );
    }
}