use std::collections::{BTreeSet, LinkedList};
use std::ops::Range;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicU16;
use std::sync::atomic::Ordering::Relaxed;
use bincode::{config::Configuration, Decode, Encode};
use log::{debug, info, warn};
use tokio::select;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::encoding::{HandlePackage};
use crate::ip::{IPLayer, IPPackage};
use crate::tcp::TCPPackage::{Data, Header, RttResponse, RttRequest, PeerVacant, Sack};
use crate::tcp::TCPState::{Receiving, Sending};

type BinaryData = Vec<u8>;

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub enum TCPPackage {
    PeerVacant,
    Sack(SackPackage),
    Data(DataPackage),
    Header(HeaderPackage),
    RttRequest(RTTPackage),
    RttResponse(RTTPackage),
}

impl Into<IPPackage> for TCPPackage {
    fn into(self) -> IPPackage {
        let encoded_package = bincode::encode_to_vec(&self, Configuration::standard()).unwrap();
        IPPackage::new(encoded_package)
    }
}

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct SackPackage {
    pub missing_ranges: Vec<Range<u16>>,
    pub largest_confirmed_sequence_id: Option<u16>,
}

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct DataPackage {
    pub sequence_id: u16,
    pub offset: u32,
    pub data: Vec<u8>,
}

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct HeaderPackage {
    pub sequence_count: u16,
    pub data_length: u32,
}

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct RTTPackage {
    pub rtt_start_millis: u16,
}

pub struct TCPLayer {
    send_package_sender: Sender<BinaryData>,
    recv_package_receiver: Receiver<BinaryData>,
}

#[derive(Debug)]
enum TCPState {
    Ready,
    Sending(TCPSendingStatus),
    Receiving(TCPReceivingStatus),
}


#[derive(Debug)]
pub struct TCPSendingStatus {
    pub data_sending: Vec<u8>,
    pub next_package_to_send: Option<TCPPackage>,
    pub sequence_missing: BTreeSet<u16>,
    pub largest_confirmed_sequence_id: Option<u16>,
    pub last_send_segment_id: Option<u16>,
    pub sequence_count: u16,
}

impl TCPSendingStatus {
    fn set_next_send_package(&mut self, sequence_length: usize) {
        debug!("We are sender, we should sending something");
        let (new_segment_id, new_package) = match self.last_send_segment_id {
            None => {
                debug!("we have not sending header, sending header...");
                (0, Header(HeaderPackage {
                    sequence_count: self.sequence_count,
                    data_length: self.data_sending.len() as _,
                }))
            }
            Some(sent_segment_id) => {
                if self.sequence_missing.is_empty() {
                    debug!("Great, no lost packages");
                    let next_segment_id = sent_segment_id + 1;
                    if next_segment_id >= self.sequence_count {
                        debug!("we have sent all the data, return");
                        self.next_package_to_send = None;
                        return;
                    }
                    (next_segment_id, Data(DataPackage {
                        sequence_id: next_segment_id,
                        offset: sent_segment_id as u32 * sequence_length as u32, // we use old segment id to calculate offset
                        data: self.data_sending[sent_segment_id as usize * sequence_length as usize..].iter().take(sequence_length).cloned().collect(),
                    }))
                } else {
                    debug!("sending lost packages");
                    let next_segment_id = self.sequence_missing.iter().next().unwrap().clone();
                    if next_segment_id == 0 {
                        (0, Header(HeaderPackage {
                            sequence_count: self.sequence_count,
                            data_length: self.data_sending.len() as _,
                        }))
                    } else {
                        (next_segment_id, Data(DataPackage {
                            sequence_id: next_segment_id,
                            offset: (next_segment_id - 1) as u32 * sequence_length as u32,
                            data: self.data_sending[(next_segment_id - 1) as usize * sequence_length as usize..].iter().take(sequence_length).cloned().collect(),
                        }))
                    }
                }
            }
        };
        self.next_package_to_send = Some(new_package);
        self.last_send_segment_id = Some(new_segment_id);
    }

    fn completed(&self) -> bool {
        self.sequence_missing.is_empty() && self.largest_confirmed_sequence_id == Some(self.sequence_count)
    }
}

#[derive(Debug)]
pub struct TCPReceivingStatus {
    pub data_received: Vec<u8>,
    pub range_ack: LinkedList<Range<u16>>,
    pub sequence_count: u16,
}

impl TCPReceivingStatus {
    pub fn set_ack(&mut self, sequence_id: u16) {
        let mut cursor = self.range_ack.cursor_front_mut();
        while let Some(range) = cursor.current() {
            if range.end >= sequence_id {
                break;
            }
            cursor.move_next();
        }
        let (new_end, remove_next) = match cursor.as_cursor().current() {
            None => {
                cursor.insert_before(sequence_id..sequence_id + 1);
                (None, false)
            }
            Some(range) => {
                let mut new_end = None;
                let mut remove_next = false;
                if range.end == sequence_id {
                    new_end = Some(sequence_id + 1);
                }

                if let Some(next_range) = cursor.as_cursor().peek_next() {
                    if new_end.is_some() && next_range.start == new_end.unwrap() {
                        new_end = Some(next_range.end);
                        remove_next = true
                    }
                }
                (new_end, remove_next)
            }
        };
        if let Some(new_end) = new_end {
            cursor.current().unwrap().end = new_end;
        }
        if remove_next {
            cursor.move_next();
            cursor.remove_current();
        }
    }

    pub fn get_ack_missing(&self) -> Vec<Range<u16>> {
        let mut cursor = self.range_ack.cursor_front();
        let mut missing = Vec::new();
        while let Some(range) = cursor.current() {
            if let Some(next_range) = cursor.peek_next() {
                missing.push(range.end..next_range.start);
            }
            cursor.move_next();
        };
        missing
    }

    pub fn completed(&self) -> bool {
        // get the range end of the first range in the range_ack, if it equals to the segment_count, then it is completed
        if let Some(range) = self.range_ack.front() {
            if range.end == self.sequence_count {
                return true;
            }
        }
        return false;
    }
}

#[derive(Debug)]
pub struct TCPRTTStatus {
    rtt: AtomicU16,
}

impl TCPRTTStatus {
    fn update_rtt(&self, new_rtt: u16) {
        // this is safe because there is only one thread to call this function
        self.rtt.store((self.rtt.load(Relaxed) + new_rtt) / 2, Relaxed);
    }

    async fn get_rtt_timeout(&self, ratio: f32) {
        tokio::time::sleep(std::time::Duration::from_millis((self.rtt.load(Relaxed) as f32 * ratio * 100.0) as u64)).await;
    }

    fn get_rtt(&self) -> u16 {
        return self.rtt.load(Relaxed);
    }

    fn generate_rtt_package() -> RTTPackage {
        return RTTPackage {
            rtt_start_millis: std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().subsec_millis() as u16,
        };
    }
}

impl TCPLayer {
    pub fn new(ip: IPLayer) -> TCPLayer {
        let sequence_length: u16 = (ip.byte_in_frame - 12) as u16;
        let ip = Arc::new(ip);
        let (send_package_sender, mut send_package_receiver) = tokio::sync::mpsc::channel::<BinaryData>(1024);
        let (recv_package_sender, recv_package_receiver) = tokio::sync::mpsc::channel::<BinaryData>(1024);

        let state: Arc<Mutex<TCPState>> = Arc::new(Mutex::new(TCPState::Ready));

        let future = async move {
            let rtt_status = TCPRTTStatus {
                rtt: AtomicU16::new(55535),
            };
            let mut rtt_timeout = Box::pin(rtt_status.get_rtt_timeout(0.0));
            let mut sack_timeout = Box::pin(rtt_status.get_rtt_timeout(1.5));
            let mut sack_timeout_count = 0;
            loop {
                let state = state.clone();
                let state_for_package_to_send_future = state.clone();
                let ip_for_package_to_send_future = ip.clone();
                let package_to_send_future = async move {
                    let package = match &*state_for_package_to_send_future.lock().unwrap() {
                        TCPState::Ready => { None }
                        Sending(sending) => {
                            sending.next_package_to_send.clone()
                        }
                        Receiving(_) => { None }
                    };
                    if let Some(package) = package {
                        ip_for_package_to_send_future.send(package.into()).await;
                    }
                };
                let (is_ready, is_sending, is_receiving) = {
                    match *state.lock().unwrap() {
                        TCPState::Ready => { (true, false, false) }
                        Sending(_) => { (false, true, false) }
                        Receiving(_) => { (false, false, true) }
                    }
                };
                select! {
                    _ = rtt_timeout.as_mut() => {
                        info!("rtt timeout, sending rtt...");
                        ip.send(RttRequest(TCPRTTStatus::generate_rtt_package()).into()).await;
                        if is_ready {
                            info!("rtt timeout, sending peer vacant...");
                            ip.send(PeerVacant.into()).await;
                        }
                        rtt_timeout = Box::pin(rtt_status.get_rtt_timeout(1.0));
                    }
                    _ = sack_timeout.as_mut() => {
                        let (sack_to_send,need_to_receive_sack) = {
                            let guard = state.lock().unwrap();
                            match &*guard{
                                TCPState::Ready => {
                                    (None,false)
                                }
                                Receiving(receiving_status) =>{
                                    let missing = receiving_status.get_ack_missing();
                                    // each pair is two u16, so a range is 4 bytes, vector is 1 u8
                                    // largest_confirmed_sequence_id is u16 plus u8
                                    let take_count = (sequence_length - 4) / 4;
                                    let missing =  missing.into_iter().take(take_count.into()).collect();
                                    (Some(SackPackage{
                                        missing_ranges: missing,
                                        largest_confirmed_sequence_id: match receiving_status.range_ack.back(){
                                            Some(range)=>{
                                                Some(range.end - 1)
                                            }
                                            None =>{
                                                None
                                            }
                                        }
                                    }),false)
                                }
                                Sending(_)=>{
                                    (None,true)
                                }
                            }
                        };
                        if let Some(sack_to_send) = sack_to_send {
                            ip.send(Sack(sack_to_send).into()).await;
                        }
                        if need_to_receive_sack{
                            sack_timeout_count += 1;
                            warn!("sack timeout, now we have {} sack timeout",sack_timeout_count);
                        }
                        sack_timeout = Box::pin(rtt_status.get_rtt_timeout(1.5));
                    }
                    package = send_package_receiver.recv(), if is_ready => {
                        if let Some(package) = package{
                            let mut state = state.lock().unwrap();
                            let package_len  = package.len();
                            let mut sending_status = TCPSendingStatus{
                                data_sending: package,
                                sequence_missing: BTreeSet::new(),
                                last_send_segment_id: None,
                                largest_confirmed_sequence_id:None,
                                next_package_to_send: None,
                                sequence_count: ((package_len + sequence_length as usize - 1) / sequence_length as usize + 1) as u16, // the first package is header
                            };
                            sending_status.set_next_send_package(sequence_length.into());
                            info!("now we have something to send, {:?}",sending_status);
                            *state = Sending(sending_status);
                        }else{
                            return;
                        }
                    },
                    _ = package_to_send_future, if is_sending => {
                        match &mut *state.lock().unwrap(){
                            Sending(sending)=>{
                                info!("we are sending the package, {:?}",sending.next_package_to_send);
                                sending.set_next_send_package(sequence_length.into());
                            }
                            _ =>{
                                unreachable!();
                            }
                        }
                    },
                    package = ip.receive() =>{
                        let package = bincode::decode_from_slice(&package.data, Configuration::standard()).unwrap();
                        info!("received package, {:?}",package);
                        match package {
                            TCPPackage::PeerVacant => {
                                let mut guard = state.lock().unwrap();
                                match &*guard{
                                    Sending(_)=>{
                                        *guard = TCPState::Ready;
                                    }
                                    _ =>{}
                                }
                            }
                            TCPPackage::Sack(sack) => {
                                if is_sending{
                                    sack_timeout = Box::pin( rtt_status.get_rtt_timeout(1.5));
                                }
                                let mut guard = state.lock().unwrap();
                                if let Sending(sending) = &mut *guard{
                                    sending.sequence_missing.extend(sack.missing_ranges.into_iter().flat_map(|range| range));
                                    sending.largest_confirmed_sequence_id = sack.largest_confirmed_sequence_id;
                                    if sending.completed(){
                                        info!("transmit finish");
                                        *guard = TCPState::Ready;
                                    }
                                }
                            }
                            TCPPackage::Data(data) => {
                                if is_receiving{
                                    let old_state = {
                                        let mut state = state.lock().unwrap();
                                        if let Receiving(receiving_status) = &mut *state {
                                            receiving_status.data_received[data.offset as usize ..data.offset as usize + data.data.len()].copy_from_slice(&data.data);
                                            receiving_status.set_ack(data.sequence_id);
                                            if receiving_status.completed(){
                                                let old_state = std::mem::replace(&mut *state, TCPState::Ready);
                                                if let Receiving(old_state) = old_state{
                                                    Some(old_state)
                                                }else{
                                                    unreachable!();
                                                }
                                            }else{
                                                None
                                            }
                                        }else{
                                            unreachable!()
                                        }
                                    };
                                    if let Some(old_state) = old_state{
                                        let largest_confirmed_sequence_id = Some(old_state.range_ack.back().unwrap().end-1);

                                        let send_result = recv_package_sender.send(old_state.data_received).await;
                                        if send_result.is_err(){
                                            return;
                                        }
                                        ip.send(Sack(SackPackage{
                                                missing_ranges: vec![],
                                                largest_confirmed_sequence_id
                                            }).into()).await;
                                        sack_timeout = Box::pin(rtt_status.get_rtt_timeout(1.5));
                                    }
                                }
                            }
                            TCPPackage::Header(header) => {
                                if is_ready{
                                    let mut state = state.lock().unwrap();
                                    *state = Receiving(TCPReceivingStatus{
                                        data_received: vec![0; header.data_length as usize],
                                        range_ack: Default::default(),
                                        sequence_count: header.sequence_count,
                                    });
                                    info!("header received, start transmitting..., {:?}",*state);
                                }
                            }
                            TCPPackage::RttRequest(rtt) => {
                                ip.send(RttResponse(rtt).into()).await;
                            },
                            TCPPackage::RttResponse(rtt) => {
                                let now_millis = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().subsec_millis() as u16;
                                info!("rtt received, {}ms in total", (1000 + now_millis - rtt.rtt_start_millis) % 1000);
                                rtt_status.update_rtt((1000 + now_millis - rtt.rtt_start_millis) % 1000);
                            },
                        }
                    }
                }
            }
        };
        tokio::spawn(future);
        Self {
            send_package_sender,
            recv_package_receiver,
        }
    }
}

impl TCPLayer {
    pub async fn send<T>(&mut self, package: &T) where T: Decode + Encode {
        let encoded_package = bincode::encode_to_vec(&package, Configuration::standard()).unwrap();
        self.send_package_sender.send(encoded_package).await.unwrap();
    }

    pub async fn receive<T>(&mut self) -> Option<T> where T: Decode + Encode {
        let data = self.recv_package_receiver.recv().await;
        if let Some(data) = data {
            bincode::decode_from_slice(&data, Configuration::standard()).unwrap()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_ack() {
        let mut s = TCPReceivingStatus {
            data_received: vec![],
            range_ack: Default::default(),
            sequence_count: 0,
        };
        s.set_ack(0);
        s.set_ack(1);
        s.set_ack(4);
        s.set_ack(5);
        s.set_ack(9);
        s.set_ack(11);
        s.set_ack(12);
        assert_eq!(s.range_ack, std::collections::LinkedList::from([0..2, 4..6, 9..10, 11..13]));
        assert_eq!(s.get_ack_missing(), vec![2..4, 6..9, 10..11]);
        s.set_ack(2);
        assert_eq!(s.range_ack, std::collections::LinkedList::from([0..3, 4..6, 9..10, 11..13]));
        assert_eq!(s.get_ack_missing(), vec![3..4, 6..9, 10..11]);
        s.set_ack(3);
        assert_eq!(s.range_ack, std::collections::LinkedList::from([0..6, 9..10, 11..13]));
        assert_eq!(s.get_ack_missing(), vec![6..9, 10..11]);
    }
}