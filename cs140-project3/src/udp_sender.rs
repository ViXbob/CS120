fn main() {
    use std::net::UdpSocket;

    let socket = UdpSocket::bind("127.0.0.1:34242").expect("couldn't bind to address");
    let buf = [1, 2, 3, 4, 5];
    socket.send_to(&buf, "127.0.0.1:34254").expect("couldn't send data");
}