use std::collections::{BTreeSet, LinkedList};
use std::ops::Range;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicU16;
use std::sync::atomic::Ordering::Relaxed;
use bincode::{config::Configuration, Decode, Encode};
use log::{debug, info, warn};
use tokio::{select};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::encoding::{HandlePackage};
use crate::ip::{IPLayer, IPPackage};
use crate::tcp::TCPPackage::{Data, Header, RttResponse, RttRequest, PeerVacant};

type BinaryData = Vec<u8>;

#[derive(Encode, Decode, PartialEq, Debug)]
pub enum TCPPackage {
    PeerVacant,
    Sack(SackPackage),
    Data(DataPackage),
    Header(HeaderPackage),
    RttRequest(RTTPackage),
    RttResponse(RTTPackage),
}

impl Into<IPPackage> for TCPPackage{
    fn into(self) -> IPPackage {
        let encoded_package = bincode::encode_to_vec(&self,Configuration::standard()).unwrap();
        IPPackage::new(encoded_package)
    }
}

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct SackPackage {
    pub receive_ranges: Vec<Range<u16>>,
    pub largest_confirmed_sequence_id: u16,
}

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct DataPackage {
    pub sequence_id: u16,
    pub offset: u32,
    pub data: Vec<u8>,
}

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct HeaderPackage {
    pub sequence_count: u16,
    pub data_length: u32,
}

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct RTTPackage {
    pub rtt_start_millis: u16,
}

pub struct TCPLayer {
    send_package_sender: Sender<BinaryData>,
    recv_package_receiver: Receiver<BinaryData>
}

#[derive(Debug)]
enum TCPState{
    Ready,
    Sending(TCPSendingStatus),
    Receiving(TCPReceivingStatus),
}


#[derive(Debug)]
pub struct TCPSendingStatus {
    pub data_sending: Vec<u8>,
    pub peer_vacant: bool,
    pub sequence_missing: BTreeSet<u16>,
    pub largest_confirmed_sequence_id: Option<u16>,
    pub last_send_segment_id: Option<u16>,
    pub sequence_count: u16,
}

impl TCPSendingStatus {
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
        self.rtt.store((self.rtt.load(Relaxed) + new_rtt) / 2,Relaxed);
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

        let sending_status: Arc<Mutex<Option<TCPSendingStatus>>> = Arc::new(Mutex::new(None));
        let receiving_status: Arc<Mutex<Option<TCPReceivingStatus>>> = Arc::new(Mutex::new(None));

        let future = async move {
            let rtt_status = TCPRTTStatus{
                rtt: AtomicU16::new(55535),
            };
            let mut rtt_timeout = Box::pin( rtt_status.get_rtt_timeout(0.0));
            let mut sack_timeout = Box::pin( rtt_status.get_rtt_timeout(1.5));
            let mut sack_timeout_count = 0;
            let mut package_to_send = generate_next_send_package(sequence_length as _, sending_status.clone());
            let mut package_to_send_is_some = package_to_send.is_some();
            loop {
                // package_to_send will always be None, if the mode of tcp connection is receiving
                let ip = ip.clone();
                let sending_status = sending_status.clone();
                let receiving_status = receiving_status.clone();
                let sending_status_is_none = sending_status.lock().unwrap().is_none();
                let receiving_status_is_none = receiving_status.lock().unwrap().is_none();
                if package_to_send.is_none(){
                    package_to_send = generate_next_send_package(sequence_length as _, sending_status.clone());
                    package_to_send_is_some = package_to_send.is_some();
                }

                let ip_for_package_to_send_future = ip.clone();
                let package_to_send_for_package_to_send_future = package_to_send.clone();
                let package_to_send_future = async move{
                    if package_to_send_is_some {
                        log::info!("Sending {:?} to ip layer.",package_to_send_for_package_to_send_future);
                        ip_for_package_to_send_future.send(package_to_send_for_package_to_send_future.unwrap()).await;
                    }else{
                        log::info!("Data pack is None.")
                    }
                };

                debug!("package_to_send_is_some: {}", package_to_send_is_some);

                if package_to_send_is_some{
                    info!("We can push a package into IP Layer");
                }
                select! {
                    _ = rtt_timeout.as_mut() => {
                        info!("rtt timeout, sending rtt...");
                        ip.send(RttRequest(TCPRTTStatus::generate_rtt_package()).into()).await;
                        if receiving_status_is_none && sending_status_is_none{
                            info!("rtt timeout, sending peer vacant...");
                            ip.send(PeerVacant.into()).await;
                        }
                        rtt_timeout = Box::pin(rtt_status.get_rtt_timeout(1.0));
                    }
                    _ = sack_timeout.as_mut() => {
                        sack_timeout_count += 1;
                        sack_timeout = Box::pin(rtt_status.get_rtt_timeout(1.5));
                        warn!("sack timeout, now we have {} sack timeout",sack_timeout_count);
                    }
                    package = send_package_receiver.recv(), if sending_status_is_none => {
                        if let Some(package) = package{
                            let mut sending_status = sending_status.lock().unwrap();
                            let package_len  = package.len();
                            *sending_status = Some(TCPSendingStatus{
                                data_sending: package,
                                peer_vacant: false,
                                sequence_missing: BTreeSet::new(),
                                last_send_segment_id: None,
                                largest_confirmed_sequence_id:None,
                                sequence_count: ((package_len + sequence_length as usize - 1) / sequence_length as usize + 1) as u16, // the first package is header
                            });
                            info!("now we have something to send, {:?}",*sending_status);
                        }else{
                            return;
                        }
                    },
                    _ = package_to_send_future, if package_to_send_is_some => {
                        info!("we are sending the package, {:?}",package_to_send);
                        package_to_send = generate_next_send_package(sequence_length as _, sending_status.clone());
                        package_to_send_is_some = package_to_send.is_some();
                    },
                    package = ip.receive() =>{
                        let package = bincode::decode_from_slice(&package.data, Configuration::standard()).unwrap();
                        info!("received package, {:?}",package);
                        match package {
                            TCPPackage::PeerVacant => {
                                let package_finish = {
                                    let mut sending_status = sending_status.lock().unwrap();
                                    debug!("sending status: {:?}",sending_status);
                                    let package_finish = if let Some(sending_status) = sending_status.as_mut() {
                                        if sending_status.largest_confirmed_sequence_id.is_some() {
                                            true
                                        } else {
                                            sending_status.peer_vacant = true;
                                            false
                                        }
                                    } else {
                                        false
                                    };
                                    if package_finish{
                                        *sending_status = None;
                                    }
                                    package_finish
                                };
                                if package_finish {
                                    let old_receiving_status = {
                                        let mut receiving_status = receiving_status.lock().unwrap();
                                        std::mem::replace(&mut *receiving_status, None).unwrap()
                                    };
                                    info!("transmit finish");
                                    if let Err(_) = recv_package_sender.send(old_receiving_status.data_received).await {
                                        return;
                                    }
                                }
                            }
                            TCPPackage::Sack(sack) => {
                                sack_timeout = Box::pin( rtt_status.get_rtt_timeout(1.5));
                                let mut guard = sending_status.lock().unwrap();
                                let sending_status_opt = guard.as_mut();
                                let completed = if let Some(sending_status) = sending_status_opt {
                                    sending_status.sequence_missing.extend(sack.receive_ranges.into_iter().flat_map(|range| range));
                                    sending_status.largest_confirmed_sequence_id = Some(sack.largest_confirmed_sequence_id);
                                    sending_status.completed()
                                }else{
                                    false
                                };
                                if completed{
                                    *guard = None;
                                    info!("transmit finish")
                                }
                            }
                            TCPPackage::Data(data) => {
                                let old_receiving_status = {
                                    let mut receiving_status = receiving_status.lock().unwrap();
                                    let completed = if let Some(receiving_status) = receiving_status.as_mut() {
                                        receiving_status.data_received[data.offset as usize ..data.offset as usize + data.data.len()].copy_from_slice(&data.data);
                                        receiving_status.set_ack(data.sequence_id);
                                        receiving_status.completed()
                                    }else{
                                        false
                                    };
                                    if completed {
                                        info!("transmit finish");
                                        Some(std::mem::replace(&mut *receiving_status, None).unwrap())
                                    }else{
                                        None
                                    }
                                };
                                if let Some(receiving_status) = old_receiving_status {
                                    if let Err(_) = recv_package_sender.send(receiving_status.data_received).await {
                                        return;
                                    }
                                }
                            }
                            TCPPackage::Header(header) => {
                                let mut receiving_status = receiving_status.lock().unwrap();
                                if let None = *receiving_status {
                                    *receiving_status = Some(TCPReceivingStatus {
                                        data_received: vec![0; header.data_length as usize],
                                        range_ack: Default::default(),
                                        sequence_count: header.sequence_count,
                                    });
                                    info!("header received, start transmitting..., {:?}",*receiving_status);
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
    pub async fn send<T>(&mut self, package: &T) where T: Decode+Encode {
        let encoded_package = bincode::encode_to_vec(&package,Configuration::standard()).unwrap();
        self.send_package_sender.send(encoded_package).await.unwrap();
    }

    pub async fn receive<T>(&mut self) -> Option<T> where T: Decode+Encode {
        let data = self.recv_package_receiver.recv().await;
        if let Some(data) = data {
            bincode::decode_from_slice(&data,Configuration::standard()).unwrap()
        }else{
            None
        }
    }
}

fn generate_next_send_package(sequence_length: usize, sending_status: Arc<Mutex<Option<TCPSendingStatus>>>) -> Option<IPPackage> {
    debug!("generating the next package, sequence_length: {}, sending_status: {:?}",sequence_length, sending_status.lock().unwrap());
    let mut guard = sending_status.lock().unwrap();
    return match guard.as_mut() {
        None => {
            debug!("no data to send, returning None");
            None
        }
        Some(sending_status) => {
            if sending_status.peer_vacant {
                debug!("peer is vacant, we should sending something");
                let sequence_count = ((sending_status.data_sending.len() + sequence_length - 1) / sequence_length as usize + 1) as u16; // the first package is header
                let (new_segment_id, new_package) = match sending_status.last_send_segment_id {
                    None => {
                        debug!("we have not sending header, sending header...");
                        (0, Header(HeaderPackage {
                            sequence_count,
                            data_length: sending_status.data_sending.len() as _,
                        }))
                    }
                    Some(sent_segment_id) => {
                        if sending_status.sequence_missing.is_empty() {
                            debug!("Great, no lost packages");
                            let next_segment_id = sent_segment_id + 1;
                            if next_segment_id >= sequence_count {
                                debug!("we have sent all the data, return None");
                                return None;
                            }
                            (next_segment_id, Data(DataPackage {
                                sequence_id: next_segment_id,
                                offset: sent_segment_id as u32 * sequence_length as u32, // we use old segment id to calculate offset
                                data: sending_status.data_sending[sent_segment_id as usize * sequence_length as usize..].iter().take(sequence_length).cloned().collect(),
                            }))
                        }else{
                            debug!("sending lost packages");
                            let next_segment_id = sending_status.sequence_missing.iter().next().unwrap().clone();
                            if next_segment_id == 0{
                                (0, Header(HeaderPackage {
                                    sequence_count,
                                    data_length: sending_status.data_sending.len() as _,
                                }))
                            }else{
                                (next_segment_id, Data(DataPackage {
                                    sequence_id: next_segment_id,
                                    offset: (next_segment_id - 1) as u32 * sequence_length as u32,
                                    data: sending_status.data_sending[(next_segment_id-1) as usize * sequence_length as usize..].iter().take(sequence_length).cloned().collect(),
                                }))
                            }
                        }
                    }
                };
                sending_status.last_send_segment_id = Some(new_segment_id);
                Some(new_package.into())
            } else {
                debug!("peer is not vacant, returning None");
                None
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_ack() {
        let mut s = TCPReceivingStatus {
            data_received: vec![],
            range_ack: Default::default(),
            sequence_count: 0
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