use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;
use crate::encoding::{BitStore, HandlePackage, NetworkPackage};
use crate::physical::{PhysicalLayer, PhysicalPackage};
use bitvec::prelude::BitVec;
use crate::redundancy::Checksum;
use crc::{Crc, CRC_16_IBM_SDLC};

static REVC_COUNT:AtomicUsize = AtomicUsize::new(0);


pub const BYTE_IN_LENGTH: usize = 2;
pub const BYTE_IN_OFFSET: usize = 1;
pub const BYTE_IN_ENDING_AND_ACK: usize = 1;
pub const BYTE_IN_ADDRESS: usize = 2;
pub const CHECKSUM: Checksum = Checksum::CRC16(&Crc::<u16>::new(&CRC_16_IBM_SDLC));

// AckPackage
// length: BYTE_IN_LENGTH
// has_more_fragments: BYTE_IN_ENDING
// address: BYTE_IN_ADDRESS
// data: len(data)
// checksum: CHECKSUM::len()

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct AckPackage {
    pub data: Vec<u8>,
}

impl AckPackage {
    pub fn new(data: impl Iterator<Item=u8>, data_len:usize, offset: usize, has_more_fragments: bool, has_ack: bool, src: u8, dest: u8) -> Self {
        let mut package = Self {
            data: Vec::with_capacity(
                data_len + BYTE_IN_LENGTH + BYTE_IN_OFFSET + BYTE_IN_ENDING_AND_ACK + BYTE_IN_ADDRESS + CHECKSUM.len(),
            ),
        };
        package.set_len(
            data_len + BYTE_IN_LENGTH + BYTE_IN_OFFSET + BYTE_IN_ENDING_AND_ACK + BYTE_IN_ADDRESS + CHECKSUM.len(),
        );
        package.set_offset(offset);
        package.set_has_more_fragments_and_ack(has_more_fragments, has_ack);
        package.set_address(src, dest);
        package.data.extend(data);
        package.set_checksum();
        package
    }

    pub fn build_from_raw(raw_data: BitStore) -> Option<Self> {
        let package = Self {
            data: raw_data.into_vec(),
        };
        // log
        println!("recv: {:?}", package.data);
        if package.validate_checksum() {
            Some(package)
        } else {
            let count = REVC_COUNT.fetch_add(1,Relaxed)+1;
            println!("validate_checksum count: {}", count);
            None
        }
    }

    pub fn extract(&self) -> Vec<u8> {
        Vec::from(
            &self.data[BYTE_IN_LENGTH + BYTE_IN_OFFSET + BYTE_IN_ENDING_AND_ACK + BYTE_IN_ADDRESS
                ..self.data.len() - CHECKSUM.len()],
        )
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        assert!(BYTE_IN_LENGTH >= 1);
        assert!(BYTE_IN_LENGTH <= (std::mem::size_of::<usize>()));
        let len_data = &self.data[..BYTE_IN_LENGTH];
        let mut len = 0;
        for data in len_data.iter().rev() {
            len = (len << 8) + (*data as usize);
        }
        len
    }

    pub fn offset(&self) -> usize {
        assert!(BYTE_IN_LENGTH >= 1);
        assert!(BYTE_IN_LENGTH <= (std::mem::size_of::<usize>()));
        let offset_data = &self.data[BYTE_IN_LENGTH..BYTE_IN_LENGTH + BYTE_IN_OFFSET];
        let mut offset = 0;
        for data in offset_data.iter().rev() {
            offset = (offset << 8) + (*data as usize);
        }
        offset
    }

    pub fn data_len(&self) ->usize{
        self.len() - BYTE_IN_LENGTH - BYTE_IN_OFFSET - BYTE_IN_ENDING_AND_ACK - BYTE_IN_ADDRESS - CHECKSUM.len()
    }

    fn set_len(&mut self, mut len: usize) {
        assert!(BYTE_IN_LENGTH >= 1);
        assert!(BYTE_IN_LENGTH <= (std::mem::size_of::<usize>()));
        for _ in 0..BYTE_IN_LENGTH {
            self.data.push((len & 0xff) as u8);
            len >>= 8;
        }
    }

    fn set_offset(&mut self, mut offset: usize) {
        assert!(BYTE_IN_OFFSET >= 1);
        assert!(BYTE_IN_OFFSET <= (std::mem::size_of::<usize>()));
        for _ in 0..BYTE_IN_OFFSET {
            self.data.push((offset & 0xff) as u8);
            offset >>= 8;
        }
    }

    pub fn checksum(&self) -> usize {
        let len = CHECKSUM.len();
        let checksum_data = &self.data[self.data.len() - len..];
        let mut checksum = 0;
        for data in checksum_data.iter().rev() {
            checksum = (checksum << 8) + (*data as usize);
        }
        checksum
    }

    pub fn validate_checksum(&self) -> bool {
        CHECKSUM.checksum(&self.data[..self.data.len() - CHECKSUM.len()]) as usize
            == self.checksum()
    }

    fn set_checksum(&mut self) {
        let mut checksum = CHECKSUM.checksum(&self.data);
        for _ in 0..CHECKSUM.len() {
            self.data.push((checksum & 0xff) as u8);
            checksum  >>=  8;
        }
    }

    pub fn has_more_fragments(&self) -> bool {
        assert_eq!(BYTE_IN_ENDING_AND_ACK, 1);
        (self.data[BYTE_IN_LENGTH + BYTE_IN_OFFSET] & 0x01) != 0
    }

    pub fn has_ack(&self) -> bool {
        assert_eq!(BYTE_IN_ENDING_AND_ACK, 1);
        ((self.data[BYTE_IN_LENGTH + BYTE_IN_OFFSET] >> 1) & 0x01) == 1
    }

    fn set_has_more_fragments_and_ack(&mut self, has_more_fragments: bool, has_ack : bool) {
        assert_eq!(BYTE_IN_ENDING_AND_ACK, 1);
        let mask = (has_more_fragments as u8) | ((has_ack as u8) << 1);
        self.data.push(mask);
    }

    fn set_address(&mut self, src: u8, dest: u8) {
        if BYTE_IN_ADDRESS != 2 {
            unimplemented!();
        }
        self.data.push(src);
        self.data.push(dest);
    }

    pub fn address(&self) -> (u8, u8) {
        if BYTE_IN_ADDRESS != 2 {
            unimplemented!();
        }
        let src = self.data[BYTE_IN_LENGTH  + BYTE_IN_OFFSET + BYTE_IN_ENDING_AND_ACK];
        let dest = self.data[BYTE_IN_LENGTH + BYTE_IN_OFFSET + BYTE_IN_ENDING_AND_ACK + 1];
        (src, dest)
    }
}

impl NetworkPackage for AckPackage {}

pub struct AckLayer {
    pub(crate) physical: PhysicalLayer,
    pub(crate) byte_in_frame: usize,
}

impl AckLayer {
    pub fn new(physical: PhysicalLayer) -> Self {
        let byte_in_frame = physical.byte_in_frame
            - BYTE_IN_ADDRESS
            - BYTE_IN_OFFSET
            - BYTE_IN_ENDING_AND_ACK
            - BYTE_IN_LENGTH
            - CHECKSUM.len();
        Self {
            physical,
            byte_in_frame,
        }
    }

    fn make_redundancy(&self, package: AckPackage) -> BitStore {
        BitVec::from_vec(package.data)
    }

    fn erase_redundancy(&self, data: BitStore) -> Option<AckPackage> {
        AckPackage::build_from_raw(data)
    }
}

impl HandlePackage<AckPackage> for AckLayer {
    fn send(&mut self, package: AckPackage) {
        let package = PhysicalPackage {
            0: self.make_redundancy(package),
        };
        assert_eq!(package.0.len(), self.physical.byte_in_frame * 8);
        self.physical.send(package);
    }

    fn receive(&mut self) -> AckPackage {
        loop {
            let result = self.physical.receive().0;
            let result = self.erase_redundancy(result);
            if let Some(result) = result {
                return result;
            }
        }
    }

    fn receive_time_out(&mut self) -> Option<AckPackage> {
        let result = self.physical.receive_time_out();
        if let Some(package) = result {
            let data = self.erase_redundancy(package.0);
            return data;
        } else {
            return None;
        }
    }
}

impl HandlePackage<PhysicalPackage> for AckLayer {
    fn send(&mut self, package: PhysicalPackage) {
        self.physical.send(package)
    }

    fn receive(&mut self) -> PhysicalPackage {
        self.physical.receive()
    }

    fn receive_time_out(&mut self) -> Option<PhysicalPackage> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cs140_common::padding::padding;

    #[test]
    fn test_ack_package() {
        let data: Vec<_> = padding().take(100).collect();
        let package = AckPackage::new(data.iter().cloned(), 100,30, false, true, 1, 2);
        assert_eq!(
            package.len(),
            100 + BYTE_IN_ENDING_AND_ACK + BYTE_IN_LENGTH + BYTE_IN_ADDRESS + CHECKSUM.len() + BYTE_IN_OFFSET
        );
        assert_eq!(package.offset(), 30);
        assert_eq!(package.has_ack(), true);
        assert_eq!(package.has_more_fragments(), false);
        assert_eq!(package.address(), (1, 2));
        assert_eq!(package.extract(), data);

        let encoded_package = BitStore::from_vec(package.data.clone());
        assert_eq!(
            AckPackage::build_from_raw(encoded_package),
            Some(package.clone())
        );
        for index in 0..package.len() * 8{
            let mut corrupted_package = BitStore::from_vec(package.data.clone());
            let reversed = !corrupted_package[index];
            corrupted_package.set(index, reversed);
            assert_eq!(AckPackage::build_from_raw(corrupted_package), None);
        }
    }
}
