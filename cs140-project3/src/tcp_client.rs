use log::debug;
use smoltcp::socket::TcpSocket;
use smoltcp::time::Instant;
use cs140_util::tcp::tcp_stack::TCPClient;

#[tokio::main]
async fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.format_timestamp_millis().init();
    let mtu: usize = 64;
    let addr = std::net::Ipv4Addr::new(101, 32, 194, 18);
    // let addr = std::new::Ipv4Addr::new(10, 19, 75, 17);
    let mut tcp_client = TCPClient::new(mtu);
    tcp_client.connect(addr, 80, 11113);
    tcp_client.send(b"GET / HTTP/1.1\n\n\n\n\n");
    let mut tcp_active = false;
    loop {
        let timestamp = Instant::now();
        match tcp_client.iface.poll(timestamp) {
            Ok(_) => {}
            Err(e) => {
                debug!("poll error: {}", e);
            }
        }
        let socket = tcp_client.iface.get_socket::<TcpSocket>(tcp_client.tcp_handle);
        if socket.is_active() && !tcp_active {
            debug!("connected");
        } else if !socket.is_active() && tcp_active {
            debug!("disconnected");
            break;
        }
        tcp_active = socket.is_active();

        if socket.may_recv() {
            let data = socket
                .recv(|data| {
                    let mut data = data.to_owned();
                    if !data.is_empty() {
                        debug!(
                            "recv data: {:?}",
                            std::str::from_utf8(data.as_ref()).unwrap_or("(invalid utf8)")
                        );
                        data = data.split(|&b| b == b'\n').collect::<Vec<_>>().concat();
                        data.reverse();
                        data.extend(b"\n");
                    }
                    (data.len(), data)
                })
                .unwrap();
            if socket.can_send() && !data.is_empty() {
                debug!(
                    "send data: {:?}",
                    std::str::from_utf8(data.as_ref()).unwrap_or("(invalid utf8)")
                );
                socket.send_slice(&data[..]).unwrap();
            }
        } else if socket.may_send() {
            debug!("close");
            socket.close();
        }
    }
}

#[cfg(test)]
mod tests{
    use tokio::io;
    use tokio::net::TcpStream;
    use super::*;
    #[tokio::test]
    async fn tcp_test() {
        let mut stream = TcpStream::connect("10.19.75.4:11113").await.unwrap();
        stream.try_write(b"GET / HTTP/1.1\n\n\n\n\n").unwrap();
        let mut buf = Vec::with_capacity(4096);
        loop {
            stream.readable().await.unwrap();
            match stream.try_read_buf(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let buf : Vec<u8> = buf.iter().take(n).map(|x| *x).collect();
                    println!("receive {:?}\nlen: {}", std::str::from_utf8(buf.as_ref()).unwrap_or("(invalid utf8)"), n);
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    println!("error");
                    break;
                }
            }
        }
    }
}