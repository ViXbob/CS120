use crate::encoding::HandlePackage;
use crate::ip::*;
use crate::physical::*;
use crate::ack::ack::{AckPackage, AckLayer};
use cs140_common::padding;

pub enum AckState {
    FrameDetection,
    Tx(usize, AckPackage),
    TxWaitingAck(usize, AckPackage),
    Rx(AckPackage),
    TxAck,
}

pub struct AckStateMachine {
    redundancy_layer : AckLayer,
    pend_to_send : Vec<u8>,
    state: AckState,
}

const FREQUENCY: &'static [f32] = &[1000.0, 2000.0, 3000.0, 4000.0, 5000.0, 6000.0, 7000.0, 8000.0, 9000.0, 10000.0, 11000.0, 12000.0, 13000.0, 14000.0, 15000.0, 16000.0];
const BYTE_IN_FRAME : usize = 72;

// impl AckStateMachine {
//     pub fn new() -> Self {
//         let physical_layer = PhysicalLayer::new(FREQUENCY, BYTE_IN_FRAME);
//         let redundancy_layer = AckLayer::new(physical_layer);
//         let pend_to_send : Vec<u8> = Vec::new();
//         let state = AckState::FrameDetection;
//         AckStateMachine {
//             redundancy_layer,
//             pend_to_send,
//             state,
//         }
//     }
//     pub fn append(&mut self, data: impl Iterator<Item=u8>) {
//         self.pend_to_send.extend(data);
//     }
//     pub fn work(&mut self) {
//         loop {
//             let now_state = self.state;
//             self.state = match now_state {
//                 AckState::FrameDetection => {
//                     if self.pend_to_send.is_empty() {
//                         let package = self.redundancy_layer.receive();
//                         // if detection succeeds
//                         AckState::Rx(package)
//                     } else {
//                         let byte_in_frame = self.redundancy_layer.byte_in_frame;
//                         let package  = if self.pend_to_send.len() <= byte_in_frame {
//                             AckPackage::new(self.pend_to_send.iter().cloned().chain(padding::padding()).take(byte_in_frame), byte_in_frame, false, 0, 0)
//                         } else {
//                             AckPackage::new(self.pend_to_send.iter().take(byte_in_frame).cloned(), byte_in_frame, true, 0, 0)
//                         };
//                         AckState::Tx(package)
//                     }
//                     AckState::FrameDetection
//                 },
//                 AckState::Tx(package) => {
//                     self.redundancy_layer.send(package.clone());
//                     AckState::TxWaitingAck(package)
//                 },
//                 AckState::TxWaitingAck(package) => {
//                     let ack_package = self.redundancy_layer.receive();
//                     // if detection succeeds
//                     AckState::FrameDetection
//                     // else
//                     // AckState::Tx(package)
//                 },
//                 AckState::Rx(package) => {
//
//                 },
//                 AckState::TxAck => {
//
//                 }
//             }
//
//         }
//     }
// }