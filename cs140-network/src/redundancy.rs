use async_trait::async_trait;
use crc::{Crc, CRC_16_IBM_SDLC};

use crate::encoding::{BitStore, HandlePackage, NetworkPackage};
use crate::physical::{PhysicalLayer, PhysicalPackage};

pub enum Checksum {
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

pub const BYTE_IN_LENGTH: usize = 2;
pub const BYTE_IN_ENDING: usize = 1;
pub const CHECKSUM: Checksum = Checksum::CRC16(&Crc::<u16>::new(&CRC_16_IBM_SDLC));
pub const BYTE_IN_ADDRESS: usize = 2;

// RedundancyPackage
// length: BYTE_IN_LENGTH
// has_more_fragments: BYTE_IN_ENDING
// address: BYTE_IN_ADDRESS
// data: len(data)
// checksum: CHECKSUM::len()

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct RedundancyPackage {
    pub data: Vec<u8>,
}

impl RedundancyPackage {
    pub fn new(data: impl Iterator<Item=u8>, data_len: usize, has_more_fragments: bool, src: u8, dest: u8) -> Self {
        let package_length = data_len + BYTE_IN_LENGTH + BYTE_IN_ENDING + BYTE_IN_ADDRESS + CHECKSUM.len();
        let mut package = Self {
            data: Vec::with_capacity(package_length),
        };
        package.set_package_length(package_length);
        package.set_has_more_fragments(has_more_fragments);
        package.set_address(src, dest);
        package.data.extend(data);
        package.set_checksum();
        package
    }

    pub fn from_physical(package: PhysicalPackage) -> Option<Self> {
        let bits:BitStore = package.into();
        let package = Self {
            data: bits.into_vec(),
        };
        if package.validate_checksum() {
            Some(package)
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        assert!(BYTE_IN_LENGTH >= 1);
        assert!(BYTE_IN_LENGTH <= (std::mem::size_of::<usize>()));
        let len_data = &self.data[..BYTE_IN_LENGTH];
        let mut len = 0;
        for data in len_data.iter().rev() {
            len = (len << 8) + (*data as usize);
        }
        len - BYTE_IN_LENGTH - BYTE_IN_ENDING - BYTE_IN_ADDRESS - CHECKSUM.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn set_package_length(&mut self, mut len: usize) {
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
        if self.data.len() < BYTE_IN_LENGTH+BYTE_IN_ENDING + BYTE_IN_ADDRESS + CHECKSUM.len() {
            return false;
        }
        CHECKSUM.checksum(&self.data[..self.data.len() - CHECKSUM.len()]) as usize
            == self.checksum()
    }

    fn set_checksum(&mut self) {
        let mut checksum = CHECKSUM.checksum(&self.data);
        for _ in 0..CHECKSUM.len() {
            self.data.push((checksum & 0xff) as u8);
            checksum >>= 8;
        }
    }

    pub fn has_more_fragments(&self) -> bool {
        assert_eq!(BYTE_IN_ENDING, 1);
        self.data[BYTE_IN_LENGTH] != 0
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
    pub fn data(&self)-> &[u8] {
        let start = BYTE_IN_LENGTH + BYTE_IN_ENDING + BYTE_IN_ADDRESS;
        let end = self.data.len() - CHECKSUM.len();
        &self.data[start..end]
    }
}

impl NetworkPackage for RedundancyPackage {}

pub struct RedundancyLayer {
    physical: PhysicalLayer,
    pub(crate) byte_in_frame: usize,
}

impl RedundancyLayer {
    pub fn new(physical: PhysicalLayer) -> Self {
        let byte_in_frame = physical.max_package_byte()
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
        BitStore::from_vec(package.data)
    }

    fn erase_redundancy(&self, data: PhysicalPackage) -> Option<RedundancyPackage> {
        RedundancyPackage::from_physical(data)
    }
}

#[async_trait]
impl HandlePackage<RedundancyPackage> for RedundancyLayer {
    async fn send(&mut self, package: RedundancyPackage) {
        let package = self.make_redundancy(package).into();
        self.physical.send(package).await;
    }

    async fn receive(&mut self) -> RedundancyPackage {
        loop {
            let result = self.physical.receive().await;
            let result = self.erase_redundancy(result);
            if let Some(result) = result {
                return result;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use cs140_common::padding::padding;

    use super::*;

    #[test]
    fn test_redundancy_package() {
        let data: Vec<_> = padding().take(100).collect();
        let package = RedundancyPackage::new(data.iter().cloned(), 100, false, 1, 2);
        assert_eq!(
            package.len(),
            100
        );
        assert_eq!(package.has_more_fragments(), false);
        assert_eq!(package.address(), (1, 2));
        assert_eq!(package.data(), &data);

        let encoded_package = BitStore::from_vec(package.data.clone());
        assert_eq!(
            RedundancyPackage::from_physical(PhysicalPackage::from(encoded_package)),
            Some(package.clone())
        );
        for index in 0..package.len() * 8 {
            let mut corrupted_package = BitStore::from_vec(package.data.clone());
            let reversed = !corrupted_package[index];
            corrupted_package.set(index, reversed);
            assert_eq!(RedundancyPackage::from_physical(PhysicalPackage::from(corrupted_package)), None);
        }
    }
}
