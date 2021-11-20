use std::thread::sleep;
use crate::encoding::HandlePackage;
use crate::ip::*;
use crate::physical::*;
use crate::ack::ack::{AckPackage, AckLayer};
use cs140_common::padding;
use log::{trace, debug, info, warn, error};
use tokio::time::error::Elapsed;

pub enum AckState {
    FrameDetection,
    Tx(AckPackage),
    Rx(AckPackage),
    TxAck,
}

pub struct AckStateMachine {
    ack_layer: AckLayer,
    tx_offset: usize,
    tx: Vec<u8>,
    rx_offset: usize,
    pub rx: Vec<u8>,
    state: AckState,
    address: u8,
}

const TIME_OUT: std::time::Duration = std::time::Duration::from_millis(1000);
const ACK_TIME_OUT: std::time::Duration = std::time::Duration::from_millis(150);


const FREQUENCY: &'static [f32] = &[1000.0, 2000.0, 3000.0, 4000.0, 5000.0, 6000.0, 7000.0, 8000.0, 9000.0, 10000.0, 11000.0, 12000.0, 13000.0, 14000.0, 15000.0, 16000.0];
// const FREQUENCY: &'static [f32] = &[1000.0, 2000.0, 3000.0, 4000.0, 5000.0, 6000.0, 7000.0, 8000.0, 9000.0, 10000.0, 11000.0, 12000.0, 13000.0, 14000.0, 15000.0, 16000.0];
// const FREQUENCY: &'static [f32] = &[1000.0, 2000.0, 3000.0, 4000.0, 5000.0, 6000.0, 7000.0, 8000.0, 9000.0, 10000.0, 11000.0, 12000.0];
// const FREQUENCY: &'static [f32] = &[1000.0, 2000.0, 3000.0, 4000.0, 5000.0, 6000.0, 7000.0, 8000.0];
// const FREQUENCY: &'static [f32] = &[1000.0, 2000.0, 3000.0, 4000.0];
const BYTE_IN_FRAME : usize = 48;
const LINK_ERROR_THRESHOLD: usize = 15;

impl AckStateMachine {
    pub fn new(input_device: usize, output_device: usize, address: u8) -> Self {
        let physical_layer = PhysicalLayer::new_with_specific_device(FREQUENCY, BYTE_IN_FRAME, input_device,output_device);
        // let physical_layer = PhysicalLayer::new(FREQUENCY, BYTE_IN_FRAME);
        let ack_layer = AckLayer::new(physical_layer);
        let tx_offset = 0;
        let tx : Vec<u8> = Vec::new();
        let rx_offset = 0;
        let rx : Vec<u8> = Vec::new();
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
                        let begin_index = self.tx_offset * byte_in_frame;
                        let package  = if begin_index + byte_in_frame >= self.tx.len() {
                            AckPackage::new(self.tx.iter().skip(begin_index).cloned().chain(padding::padding()).take(byte_in_frame), self.tx.len() - begin_index, self.tx_offset,false, false, 0, 0)
                        } else {
                            AckPackage::new(self.tx.iter().skip(begin_index).take(byte_in_frame).cloned(), byte_in_frame, self.tx_offset, true, false, self.address, 0)
                        };
                        AckState::Tx(package)
                    }
                },
                AckState::Tx(package) => {
                    debug!("package {} need to be sent!", self.tx_offset);
                    trace!("{:?}", package.data);
                    let mut accumulate_lost_ack = 0;
                    loop {
                        // self.ack_layer.physical.push_warm_up_data();
                        self.ack_layer.send(package.clone()).await;
                        trace!("send: {:?}", package.data);
                        // self.ack_layer.physical.push_warm_up_data(25);
                        // let ack_package: Option<AckPackage> = self.ack_layer.receive_time_out();
                        let ack_package = loop{
                            let package = tokio::time::timeout(ACK_TIME_OUT,self.ack_layer.physical.receive()).await;
                                if let Ok(package) = package {
                                    let package = AckPackage{data: package.0.into_vec()};
                                    if package.address().0 != self.address{
                                        break Some(package)
                                    }
                                } else {
                                    break None
                                };
                        };
                        if let Some(ack_package) = ack_package {
                            accumulate_lost_ack = 0;
                            trace!("recv: {:?}", ack_package.data);
                            let has_ack = ack_package.has_ack();
                            let now_offset = ack_package.offset();
                            if has_ack && (now_offset >= self.tx_offset) {
                                debug!("package {} was sent successfully!", self.tx_offset);
                                self.tx_offset = now_offset + 1;
                                break;
                            }
                        } else {
                            accumulate_lost_ack += 1;
                            if accumulate_lost_ack > LINK_ERROR_THRESHOLD {
                                println!("Link Error!");
                                return;
                            }
                        }
                    }
                    // self.tx_offset += 1;
                    if !package.has_more_fragments() { return; }
                    AckState::FrameDetection
                },
                AckState::Rx(package) => {
                    if package.has_ack() {
                        AckState::FrameDetection
                    } else if self.rx_offset != package.offset() {
                        AckState::TxAck
                    } else {
                        self.rx_offset = package.offset() + 1;
                        self.rx.extend(package.extract().iter().take(package.data_len()));
                        debug!("package {} was received successfully!", self.rx_offset);
                        if !package.has_more_fragments() { return; }
                        AckState::TxAck
                    }
                },
                AckState::TxAck => {
                    // std::thread::sleep(std::time::Duration::from_millis(25));
                    // self.ack_layer.physical.push_warm_up_data(25);
                    self.ack_layer.send(AckPackage::new(padding::padding().take(byte_in_frame), 0, self.rx_offset - 1, false, true, 0, 0)).await;
                    debug!("the acknowledgment of package {} was sent!", self.rx_offset - 1);
                    AckState::FrameDetection
                }
            }
        }
    }
}