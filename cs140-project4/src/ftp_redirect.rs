use std::net::IpAddr;
use std::str::FromStr;
use std::sync::Arc;
use pcap::{Device, Capture};
fn main() {
    println!("{:?}", Device::list().unwrap());
    let list = Device::list().unwrap();
    let mut device_: Device = Device::lookup().unwrap();
    for device in list {
        let addresses =  device.addresses.clone();
        let mut availavle = false;
        for address in addresses {
            if address.addr == IpAddr::from_str("10.19.72.77").unwrap() {
                availavle = true;
            }
        }
        if availavle {
            device_ = device.clone();
            break;
        }
    }
    let mut cap = device_.open().unwrap();
    cap.filter("host 10.19.95.147", false).unwrap();
    while let Ok(packet) = cap.next() {
        println!("received packet! {:?}", packet);
    }
}
