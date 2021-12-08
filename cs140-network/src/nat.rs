use std::net::{UdpSocket, Ipv4Addr, SocketAddrV4};
use std::str::FromStr;
use crate::tcp::{TCPPackage, TCPLayer};

const LISTEN_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(50);

pub struct NatServer {
    listen_address: str,
    listen_port: u16,
    connect_address: str,
    connect_port: u16,
}

impl NatServer {
    pub fn new(listen_address: &str, listen_port: u16, connect_address: &str, connect_port: u16) -> Self {
        NatServer {
            listen_address: listen_address.cloned(),
            listen_port,
            connect_address: connect_address.cloned(),
            connect_port,
        }
    }

    fn address() -> SocketAddrV4 {

    }

    fn check_icmp(package: &TCPPackage) -> bool {
        todo!()
    }

    fn check_udp(package: &TCPPackage) -> bool {
        todo!()
    }

    fn check_tcp(package: &TCPPackage) -> bool {
        todo!()
    }

    fn listen(address: &str, port: u16) {
        let ipv4: Ipv4Addr = Ipv4Addr::from_str(address).expect("invalid ipv4 address");
        let address: SocketAddrV4 = SocketAddrV4::new(ipv4, port);
        let socket = UdpSocket::bind(address).expect("couldn't bind to this address");
        socket.set_read_timeout(Some(LISTEN_TIMEOUT));
        loop {

        }
    }

    pub fn work(&self) {
        let listener = std::thread::spawn(listen(self.listen_address.cloned(), self.listen_port));
        let connector = std::thread::spawn(connect(self.connect_address.cloned(), self.connect_port));
    }
}