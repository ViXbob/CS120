use std::net::UdpSocket;

fn main() {
    let socket = UdpSocket::bind("127.0.0.1:34254").expect("couldn't bind to this address");
    let mut buf = [0; 10];
    let (number_of_bytes, src_addr) = socket.peek_from(&mut buf)
        .expect("Didn't receive data");
    let filled_buf = &mut buf[..number_of_bytes];
    println!("{:?}", filled_buf);
}