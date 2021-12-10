use tokio::net::UdpSocket;
use cs140_util::icmp::IcmpSocket;
use cs140_util::rpc::CS120Socket;
use pnet::packet::icmp::echo_reply::EchoReplyPacket;

#[tokio::main]
async fn main() {
    // let socket = UdpSocket::bind("10.19.73.32:34241").await.unwrap();
    // let len = socket.send_to(b"hello world", "10.19.75.77:28888").await.unwrap();
    // println!("len:{}", len);
    // std::thread::park();
    let socket = IcmpSocket::new();
    let mut buf = [0u8; 1024];
    let result = socket.recv_from_addr(&mut buf).await;
    println!("{:?}", result);
    // loop {
    //     if let Ok((len, addr)) = socket.recv_from_addr(&mut buf).await {
    //         if let Some(icmp_package) = EchoReplyPacket::new(&mut buf[..]) {
    //             println!("len: {}, addr: {:?}", len, addr);
    //             println!("icmp reply: {:?}", icmp_package);
    //         }
    //     }
    // }
}