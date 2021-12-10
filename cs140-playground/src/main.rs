use tokio::net::UdpSocket;

#[tokio::main]
async fn main() {
    let socket = UdpSocket::bind("10.19.73.32:34241").await.unwrap();
    let len = socket.send_to(b"hello world", "10.19.75.77:28888").await.unwrap();
    println!("len:{}", len);
    std::thread::park();
}