use smoltcp::time::Instant;
use cs140_util::tcp::tcp_stack::TCPClient;

#[tokio::main]
async fn main() {
    let mtu: usize = 64;
    let addr = std::net::Ipv4Addr::new(101, 32, 194, 18);
    let mut tcp_client = TCPClient::new(mtu);
    tcp_client.connect(addr, 1111, 11112);
    let buf: Vec<u8> = vec![1, 2, 3, 4];
    tcp_client.send(buf.as_slice());
    loop {
        let timestamp = Instant::now();
        tcp_client.iface.poll(timestamp);
    }
}