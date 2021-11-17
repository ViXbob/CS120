use crate::encoding::HandlePackage;
use crate::ip::*;
use crate::physical::*;
use crate::ack::ack::{AckPackage, AckLayer};
use cs140_common::padding;

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
    rx: Vec<u8>,
    state: AckState,
}

const FREQUENCY: &'static [f32] = &[1000.0, 2000.0, 3000.0, 4000.0, 5000.0, 6000.0, 7000.0, 8000.0, 9000.0, 10000.0, 11000.0, 12000.0, 13000.0, 14000.0, 15000.0, 16000.0];
const BYTE_IN_FRAME : usize = 72;

impl AckStateMachine {
    pub fn new() -> Self {
        let physical_layer = PhysicalLayer::new(FREQUENCY, BYTE_IN_FRAME);
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
        }
    }
    pub fn append(&mut self, data: impl Iterator<Item=u8>) {
        self.tx.extend(data);
    }
    pub fn work(&mut self) {
        loop {
            let now_state = &self.state;
            self.state = match now_state {
                AckState::FrameDetection => {
                    if self.tx.is_empty() {
                        let package = self.ack_layer.receive_time_out();
                        if let Some(package) = package {
                            AckState::Rx(package)
                        } else {
                            AckState::FrameDetection
                        }
                    } else {
                        let byte_in_frame = self.ack_layer.byte_in_frame;
                        let begin_index = self.tx_offset * byte_in_frame;
                        let package  = if begin_index + byte_in_frame >= self.tx.len() {
                            AckPackage::new(self.tx.iter().skip(begin_index).cloned().chain(padding::padding()).take(byte_in_frame), byte_in_frame, self.tx_offset,false, false, 0, 0)
                        } else {
                            AckPackage::new(self.tx.iter().skip(begin_index).take(byte_in_frame).cloned(), byte_in_frame, self.tx_offset, true, false, 0, 0)
                        };
                        AckState::Tx(package)
                    }
                },
                AckState::Tx(package) => {
                    loop {
                        self.ack_layer.send(package.clone());
                        let ack_package: Option<AckPackage> = self.ack_layer.receive_time_out();
                        if let Some(ack_package) = ack_package {
                            if ack_package.has_ack() {
                                break;
                            }
                        }
                    }
                    self.tx_offset += 1;
                    AckState::FrameDetection
                },
                AckState::Rx(package) => {
                    if package.has_ack() || self.rx_offset + 1 != package.offset() {
                        AckState::FrameDetection
                    } else {
                        self.rx_offset = package.offset();
                        self.rx.extend(package.data.iter());
                        AckState::TxAck
                    }
                },
                AckState::TxAck => {
                    let payload : Vec<u8> = Vec::new();
                    self.ack_layer.send(AckPackage::new(payload.iter().cloned(), 0, 0, false, true, 0, 0));
                    AckState::FrameDetection
                }
            }
        }
    }
}