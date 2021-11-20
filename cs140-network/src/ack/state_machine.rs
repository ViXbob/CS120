use std::cmp::max;
use std::thread::sleep;
use crate::encoding::HandlePackage;
use crate::ip::*;
use crate::physical::*;
use crate::ack::ack::{AckPackage, AckLayer};
use cs140_common::padding;
use log::{trace, debug, info, warn, error};
use tokio::time::error::Elapsed;

const TIME_OUT: std::time::Duration = std::time::Duration::from_millis(1000);
const ACK_TIME_OUT: std::time::Duration = std::time::Duration::from_millis(3000);


const FREQUENCY: &'static [f32] = &[1000.0, 2000.0, 3000.0, 4000.0, 5000.0, 6000.0, 7000.0, 8000.0, 9000.0, 10000.0, 11000.0, 12000.0, 13000.0, 14000.0, 15000.0, 16000.0];
// const FREQUENCY: &'static [f32] = &[1000.0, 2000.0, 3000.0, 4000.0, 5000.0, 6000.0, 7000.0, 8000.0, 9000.0, 10000.0, 11000.0, 12000.0, 13000.0, 14000.0, 15000.0, 16000.0];
// const FREQUENCY: &'static [f32] = &[1000.0, 2000.0, 3000.0, 4000.0, 5000.0, 6000.0, 7000.0, 8000.0, 9000.0, 10000.0, 11000.0, 12000.0];
// const FREQUENCY: &'static [f32] = &[1000.0, 2000.0, 3000.0, 4000.0, 5000.0, 6000.0, 7000.0, 8000.0];
// const FREQUENCY: &'static [f32] = &[1000.0, 2000.0, 3000.0, 4000.0];
const SIZE: usize = 6250;
const BYTE_IN_FRAME : usize = 48;
const CONTENT_IN_FRAME: usize = BYTE_IN_FRAME - 8;
const LINK_ERROR_THRESHOLD: usize = 15;
const WINDOW_SIZE: usize = 180;
const TOTAL: usize = (SIZE + CONTENT_IN_FRAME - 1) / CONTENT_IN_FRAME;

pub enum AckState {
    FrameDetection,
    Tx(Vec<AckPackage>),
    Rx(AckPackage),
    TxAck,
}

pub struct AckStateMachine {
    ack_layer: AckLayer,
    tx_offset: usize,
    tx: Vec<u8>,
    rx_offset: usize,
    pub rx: [Option<Vec<u8>>; TOTAL],
    state: AckState,
    address: u8,
}

impl AckStateMachine {
    pub fn new(input_device: usize, output_device: usize, address: u8) -> Self {
        let physical_layer = PhysicalLayer::new_with_specific_device(FREQUENCY, BYTE_IN_FRAME, input_device,output_device);
        // let physical_layer = PhysicalLayer::new(FREQUENCY, BYTE_IN_FRAME);
        let ack_layer = AckLayer::new(physical_layer);
        let tx_offset = 0;
        let tx : Vec<u8> = Vec::new();
        let rx_offset = 0;
        const rx_element: Option<Vec<u8>> = None;
        let rx = [rx_element; TOTAL];
        let state = AckState::FrameDetection;
        AckStateMachine {
            ack_layer,
            tx_offset,
            tx,
            rx_offset,
            rx,
            state,
            address
        }
    }
    pub fn append(&mut self, data: impl Iterator<Item=u8>) {
        self.tx.extend(data);
    }
    pub async fn work(&mut self) {
        let byte_in_frame = self.ack_layer.byte_in_frame;
        loop {
            let now_state = &self.state;
            self.state = match now_state {
                AckState::FrameDetection => {
                    if self.tx.is_empty() {
                        let package = tokio::time::timeout(TIME_OUT,self.ack_layer.receive()).await;
                        if let Ok(package) = package {
                            AckState::Rx(package)
                        } else {
                            AckState::FrameDetection
                        }
                    } else {
                        let mut packages : Vec<_> = Vec::new();
                        packages = (0..WINDOW_SIZE).map(|index| {
                            let begin_index = (self.tx_offset + index) * byte_in_frame;
                            if begin_index >= self.tx.len() { return None }
                            Some(if begin_index + byte_in_frame >= self.tx.len() {
                                AckPackage::new(self.tx.iter().skip(begin_index).cloned().chain(padding::padding()).take(byte_in_frame), self.tx.len() - begin_index, self.tx_offset + index,false, false, 0, 0)
                            } else {
                                AckPackage::new(self.tx.iter().skip(begin_index).take(byte_in_frame).cloned(), byte_in_frame, self.tx_offset + index, true, false, self.address, 0)
                            }                            )
                        }).filter(|x|x.is_some()).map(|x|x.unwrap()).collect();
                        AckState::Tx(packages)
                    }
                },
                AckState::Tx(packages) => {
                    debug!("packages {:?} need to be sent!", (self.tx_offset + 0..self.tx_offset + WINDOW_SIZE));
                    trace!("package {} need to be sent!", self.tx_offset);
                    // trace!("{:?}", package.data);
                    let mut accumulate_lost_ack = 0;
                    loop {
                        // self.ack_layer.physical.push_warm_up_data();
                        for (index, package) in packages.iter().enumerate() {
                            self.ack_layer.send(package.clone()).await;
                            debug!("send package {}: {:?}", index, package.data);
                        }
                        // self.ack_layer.physical.push_warm_up_data(25);
                        // let ack_package: Option<AckPackage> = self.ack_layer.receive_time_out();
                        debug!("A send was finished!");

                        let (new_ack, package_count) = async{
                            let mut total_time_out = ACK_TIME_OUT;
                            let mut old_ack = self.tx_offset;
                            let mut package_count = 0;
                            loop {
                                let now = std::time::Instant::now();
                                let package = tokio::time::timeout(total_time_out,self.ack_layer.physical.receive()).await;
                                match package{
                                    Ok(package) => {
                                        package_count+=1;
                                        let package = AckPackage{data: package.0.into_vec()};
                                        if package.address().0 != self.address && package.has_ack() {
                                            old_ack = std::cmp::max(old_ack,package.offset());
                                        }
                                    }
                                    Err(_) => {break;}
                                }
                                let duration = now.elapsed();
                                if total_time_out <= duration { break; }
                                else { total_time_out -= duration; }
                            }
                            (old_ack, package_count)
                        }.await;


                        if package_count > 0 {
                            accumulate_lost_ack = 0;
                            debug!("a new ack is received, {}", new_ack);
                            self.tx_offset = max(self.tx_offset, new_ack);
                            break;
                        } else {
                            accumulate_lost_ack += 1;
                            if accumulate_lost_ack > LINK_ERROR_THRESHOLD {
                                println!("Link Error!");
                                return;
                            }
                        }
                    }
                    AckState::FrameDetection
                },
                AckState::Rx(package) => {
                    if package.has_ack() {
                        self.tx_offset = max(self.tx_offset, package.offset());
                        AckState::FrameDetection
                    } else {
                        if self.rx[package.offset()].is_none() {
                            self.rx[package.offset()] = Some(package.extract().into_iter().take(package.data_len()).collect());
                            debug!("package {} was received successfully!", self.rx_offset);
                            loop {
                                if self.rx[self.rx_offset].is_none() {
                                    break;
                                }
                                self.rx_offset += 1;
                                if self.rx_offset >= TOTAL {
                                    return;
                                }
                            }
                            debug!("now rx offset is {}", self.rx_offset);
                        };
                        AckState::TxAck
                    }
                },
                AckState::TxAck => {
                    // std::thread::sleep(std::time::Duration::from_millis(25));
                    // self.ack_layer.physical.push_warm_up_data(25);
                    self.ack_layer.send(AckPackage::new(padding::padding().take(byte_in_frame), 0, self.rx_offset, false, true, 0, 0)).await;
                    trace!("the acknowledgment {} was sent!", self.rx_offset);
                    AckState::FrameDetection
                }
            }
        }
    }
}