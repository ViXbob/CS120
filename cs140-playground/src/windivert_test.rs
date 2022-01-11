use windivert::{WinDivert, WinDivertFlags, WinDivertLayer};

fn main() {
    let wind = WinDivert::new("remoteAddr == 10.19.95.147", WinDivertLayer::Network, i16::MAX, WinDivertFlags::new()).unwrap();
    loop {
        let packet = wind.recv(65536).unwrap();
        println!("{:?}", packet);
    }
}