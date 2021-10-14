use crate::encoding::{BitStore, HandlePackage, NetworkPackage};
use crate::physical::{PhysicalLayer, PhysicalPackage};
use bitvec::prelude::BitVec;

use crc::{Crc, CRC_16_IBM_SDLC};

enum Checksum {
    CRC16(&'static Crc<u16>),
    CRC32(&'static Crc<u32>),
    CRC64(&'static Crc<u64>),
}

impl Checksum {
    pub fn len(&self) -> usize {
        match self {
            Checksum::CRC16(_) => 2,
            Checksum::CRC32(_) => 4,
            Checksum::CRC64(_) => 8,
        }
    }

    pub fn checksum(&self, data: &[u8]) -> usize {
        match self {
            Checksum::CRC16(c) => c.checksum(data) as usize,
            Checksum::CRC32(c) => c.checksum(data) as usize,
            Checksum::CRC64(c) => c.checksum(data) as usize,
        }
    }
}

const BYTE_IN_LENGTH: usize = 2;
const BYTE_IN_ENDING: usize = 1;
const CHECKSUM: Checksum = Checksum::CRC16(&Crc::<u16>::new(&CRC_16_IBM_SDLC));
const BYTE_IN_ADDRESS: usize = 2;

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct RedundancyPackage {
    pub data: Vec<u8>,
}

impl RedundancyPackage {
    pub fn new(data: impl Iterator<Item=u8>, data_len:usize , has_more_fragments: bool, src: u8, dest: u8) -> Self {
        let mut package = Self {
            data: Vec::with_capacity(
                data_len + BYTE_IN_LENGTH + BYTE_IN_ENDING + BYTE_IN_ADDRESS + CHECKSUM.len(),
            ),
        };
        package.set_len(
            data_len + BYTE_IN_LENGTH + CHECKSUM.len() + BYTE_IN_ENDING + BYTE_IN_ADDRESS,
        );
        package.set_has_more_fragments(has_more_fragments);
        package.set_address(src, dest);
        package.data.extend(data);
        package.set_checksum();
        package
    }

    pub fn build_from_raw(raw_data: BitStore) -> Option<Self> {
        let package = Self {
            data: raw_data.into_vec(),
        };
        if package.validate_checksum() {
            Some(package)
        } else {
            None
        }
    }

    pub fn extract(&self) -> Vec<u8> {
        Vec::from(
            &self.data[BYTE_IN_LENGTH + BYTE_IN_ENDING + BYTE_IN_ADDRESS
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

    fn set_len(&mut self, mut len: usize) {
        assert!(BYTE_IN_LENGTH >= 1);
        assert!(BYTE_IN_LENGTH <= (std::mem::size_of::<usize>()));
        for _ in 0..BYTE_IN_LENGTH {
            self.data.push((len & 0xff) as u8);
            len >>= 8;
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
        assert_eq!(BYTE_IN_ENDING, 1);
        self.data[BYTE_IN_LENGTH]!=0
    }

    fn set_has_more_fragments(&mut self, has_more_fragments: bool) {
        assert_eq!(BYTE_IN_ENDING, 1);
        self.data.push(if has_more_fragments { 1 } else { 0 });
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
        let src = self.data[BYTE_IN_LENGTH + BYTE_IN_ENDING];
        let dest = self.data[BYTE_IN_LENGTH + BYTE_IN_ENDING + 1];
        (src, dest)
    }
}

impl NetworkPackage for RedundancyPackage {}

pub struct RedundancyLayer {
    pub(crate) physical: PhysicalLayer,
    pub(crate) byte_in_frame: usize,
}

impl RedundancyLayer {
    pub fn new(physical: PhysicalLayer) -> Self {
        let byte_in_frame = physical.byte_in_frame
            - BYTE_IN_ADDRESS
            - BYTE_IN_ENDING
            - BYTE_IN_LENGTH
            - CHECKSUM.len();
        Self {
            physical,
            byte_in_frame,
        }
    }

    fn make_redundancy(&self, package: RedundancyPackage) -> BitStore {
        BitVec::from_vec(package.data)
    }

    fn erase_redundancy(&self, data: BitStore) -> Option<RedundancyPackage> {
        RedundancyPackage::build_from_raw(data)
    }
}

impl HandlePackage<RedundancyPackage> for RedundancyLayer {
    fn send(&mut self, package: RedundancyPackage) {
        let package = PhysicalPackage {
            0: self.make_redundancy(package),
        };
        assert_eq!(package.0.len(), self.physical.byte_in_frame);
        self.physical.send(package);
    }

    fn receive(&mut self) -> RedundancyPackage {
        loop {
            let result = self.physical.receive().0;
            let result = self.erase_redundancy(result);
            if let Some(result) = result {
                return result;
            }
        }
    }
}

impl HandlePackage<PhysicalPackage> for RedundancyLayer {
    fn send(&mut self, package: PhysicalPackage) {
        self.physical.send(package)
    }

    fn receive(&mut self) -> PhysicalPackage {
        self.physical.receive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cs140_common::padding::padding;

    #[test]
    fn test_redundancy_package() {
        let data: Vec<_> = padding().take(100).collect();
        let package = RedundancyPackage::new(data.clone(), 100,false, 1, 2);
        assert_eq!(
            package.len(),
            100 + BYTE_IN_ENDING + BYTE_IN_LENGTH + BYTE_IN_ADDRESS + CHECKSUM.len()
        );
        assert_eq!(package.has_more_fragments(), false);
        assert_eq!(package.address(), (1, 2));
        assert_eq!(package.extract(), data);

        let encoded_package = BitStore::from_vec(package.data.clone());
        assert_eq!(
            RedundancyPackage::build_from_raw(encoded_package),
            Some(package.clone())
        );
        for index in 0..package.len() * 8{
            let mut corrupted_package = BitStore::from_vec(package.data.clone());
            let reversed = !corrupted_package[index];
            corrupted_package.set(index, reversed);
            assert_eq!(RedundancyPackage::build_from_raw(corrupted_package), None);
        }
    }
}
