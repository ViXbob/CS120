use pcap::{Device,Capture};
fn main() {
    let main_device = Device::lookup().unwrap();
    let mut cap = Capture::from_device(main_device).unwrap()
        .promisc(true)
        .snaplen(5000)
        .open().unwrap();
    cap.filter(&"tcp dst portrange 18880-18889", true).unwrap();
    while let Ok(packet) = cap.next() {
        println!("received packet! {:?}", packet);
    }
}