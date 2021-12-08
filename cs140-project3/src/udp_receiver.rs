use std::net::UdpSocket;

fn main() {
    let socket = UdpSocket::bind("127.0.0.1:34254").expect("couldn't bind to this address");
    let mut buf = [0;2000];
    let (number_of_bytes, src_addr) = socket.peek_from(&mut buf)
        .expect("Didn't receive data");
    let filled_buf = &mut buf[..number_of_bytes];
    let string : &str = std::str::from_utf8(filled_buf).expect("couldn't convert");
    println!("{}", string);
}