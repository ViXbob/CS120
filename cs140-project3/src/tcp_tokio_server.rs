use std::net::SocketAddr;
use tokio::io::AsyncWriteExt;
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
    let data = file_io::read_bytes_from_file("INPUT.txt");
    println!("please type the listening address,");
    let mut buf: String = String::new();
    read(&mut buf);
    let addr = buf.parse::<SocketAddr>().unwrap();
    let tcp = TcpListener::bind(addr).await.unwrap();
    loop {
        let (mut socket, _) = tcp.accept().await.unwrap();
        println!("accept a new connection!");
        socket.write_all(data.as_slice()).await;
    }
}