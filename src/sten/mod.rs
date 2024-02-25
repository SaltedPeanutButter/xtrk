mod image_io;

#[derive(Debug, thiserror::Error)]
pub enum StenError {
    #[error("{0}")]
    ImageIoError(#[from] image_io::ImageIoError),

    #[error("Payload too large for container. Payload size: {0}, maximum size: {1}")]
    PayloadTooLarge(usize, usize),

    #[error("Unable to extract payload/checksum/length")]
    BadPayload,

    #[error("Checksum does not match. Payload may be corrupted.")]
    FailedChecksum,

    #[error("Failed to parse payload")]
    FailedParsing,
}

fn get_crc(crc: u32, data: &[u8]) -> u32 {
    let poly = 0xedb88320u32;
    let mut crc = !crc;
    for byte in data {
        crc ^= *byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1) * poly;
            crc = (crc >> 1) ^ mask;
        }
    }
    !crc
}

pub trait Container {
    fn as_mut_bytes(&mut self) -> &mut [u8];
    fn as_bytes(&self) -> &[u8];
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> Container for T {
    fn as_bytes(&self) -> &[u8] {
        self.as_ref()
    }

    fn as_mut_bytes(&mut self) -> &mut [u8] {
        self.as_mut()
    }
}

/// This trait is used to perform stenographic operations.
pub trait Stenable: Sized {
    /// Get raw bytes of the object.
    fn get_raw_bytes(self) -> Vec<u8>;

    /// Perform stenographic operation. Default implementation is provided.
    fn sten<C: Container>(self, container: &mut C) -> Result<(), StenError> {
        let mut payload = self.get_raw_bytes();

        // Calculate checksum and build payload
        let crc = get_crc(0, &payload).to_le_bytes(); // convert checksum to little endian bytes
        payload.extend_from_slice(&crc);

        // Prepend payload size to payload
        let container = container.as_mut_bytes();
        let payload_size = payload.len() as u32;
        let payload_size = payload_size.to_le_bytes();
        let mut new_payload = payload_size.to_vec();
        new_payload.extend_from_slice(&payload);

        // Perform size check
        let max_size = container.len() / 8; // in bytes
        let new_payload_size = new_payload.len(); // in bytes
        if new_payload_size > max_size {
            return Err(StenError::PayloadTooLarge(new_payload_size, max_size));
        }

        let mut container_byte_pos = 0; // to keep track of the container byte position
                                        // Iterate over byte of payload
        for payload_byte in new_payload {
            // Iterate over bit position of payload byte
            for payload_bit_pos in 0..8 {
                // Get payload bit
                let payload_bit = (payload_byte >> payload_bit_pos) & 1;

                // Unset the container bit
                container[container_byte_pos] &= 0xFE;

                // Set the container bit at the position with the payload bit
                container[container_byte_pos] |= payload_bit;

                // Move to the next container byte
                container_byte_pos += 1;
            }
        }

        Ok(())
    }
}

/// This trait is used to reverse stenographic operations.
pub trait Destenable: Sized {
    /// Convert raw bytes to object.
    fn from_raw_bytes(data: Vec<u8>) -> Option<Self>;

    /// Reverse stenographic operation. Default implementation is provided.
    fn desten<C: Container>(container: &C) -> Result<Self, StenError> {
        let container = container.as_bytes();

        // Validate container minimum size
        if container.len() < 32 {
            return Err(StenError::BadPayload);
        }

        // Extract payload size in bytes (first 4 bytes)
        let payload_size: usize = container[..32] // last 32 payload bits = 4 payload bytes
            .iter()
            .map(|b| b & 1) // get the last bit of each byte
            .enumerate() // pair each bit with its position
            .map(|(i, b)| (b as usize) << i) // shift the bit to its position
            .sum(); // add all the bits together

        let bytes_to_read = payload_size * 8; // each container byte store 1 bit
        let mut payload_byte = 0u8; // to build up the payload byte from each bit read
        let mut bit_read = 0; // to set bit in payload byte and to know when to move on to the next byte
        let mut payload_with_checksum = Vec::with_capacity(payload_size);

        for container_byte in container[32..32 + bytes_to_read].iter() {
            // Read the LSB of the container byte
            let container_bit = container_byte & 1;

            // Set the payload byte bit
            payload_byte |= container_bit << bit_read;

            // Move to the next bit
            bit_read += 1;

            // If we have read 8 bits, then we have a full payload byte,
            // then insert it to the payload
            if bit_read == 8 {
                payload_with_checksum.push(payload_byte);
                payload_byte = 0;
                bit_read = 0;
            }
        }

        // Extract checksum and actual payload
        let payload = payload_with_checksum[..payload_with_checksum.len() - 4].to_vec();
        let expected = u32::from_le_bytes(
            payload_with_checksum[payload_with_checksum.len() - 4..]
                .try_into()
                .unwrap(), // can unwrap since payload size is at least 4
        );

        // Calculate and compare actual checksum
        let actual = get_crc(0, &payload);
        if expected != actual {
            return Err(StenError::FailedChecksum);
        }

        // Decode payload
        let p = Self::from_raw_bytes(payload).ok_or(StenError::FailedParsing)?;
        Ok(p)
    }
}

impl Stenable for &[u8] {
    fn get_raw_bytes(self) -> Vec<u8> {
        self.to_vec()
    }
}

impl Stenable for &mut [u8] {
    fn get_raw_bytes(self) -> Vec<u8> {
        self.to_vec()
    }
}

impl<T: Stenable> Stenable for Option<T> {
    fn get_raw_bytes(self) -> Vec<u8> {
        match self {
            Some(t) => t.get_raw_bytes(),
            None => vec![],
        }
    }
}

impl<T: Destenable> Destenable for Option<T> {
    fn from_raw_bytes(data: Vec<u8>) -> Option<Self> {
        T::from_raw_bytes(data).map(Some)
    }
}

impl Stenable for Vec<u8> {
    fn get_raw_bytes(self) -> Vec<u8> {
        self
    }
}

impl Destenable for Vec<u8> {
    fn from_raw_bytes(data: Vec<u8>) -> Option<Self> {
        Some(data)
    }
}

impl Stenable for String {
    fn get_raw_bytes(self) -> Vec<u8> {
        self.into_bytes()
    }
}

impl Destenable for String {
    fn from_raw_bytes(data: Vec<u8>) -> Option<Self> {
        String::from_utf8(data).ok()
    }
}

impl Stenable for u8 {
    fn get_raw_bytes(self) -> Vec<u8> {
        vec![self]
    }
}

impl Destenable for u8 {
    fn from_raw_bytes(data: Vec<u8>) -> Option<Self> {
        data.first().copied()
    }
}

impl Stenable for u16 {
    fn get_raw_bytes(self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}

impl Destenable for u16 {
    fn from_raw_bytes(data: Vec<u8>) -> Option<Self> {
        data.try_into().map(u16::from_le_bytes).ok()
    }
}

impl Stenable for u32 {
    fn get_raw_bytes(self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}

impl Destenable for u32 {
    fn from_raw_bytes(data: Vec<u8>) -> Option<Self> {
        data.try_into().map(u32::from_le_bytes).ok()
    }
}

impl Stenable for u64 {
    fn get_raw_bytes(self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}

impl Destenable for u64 {
    fn from_raw_bytes(data: Vec<u8>) -> Option<Self> {
        data.try_into().map(u64::from_le_bytes).ok()
    }
}

impl Stenable for i8 {
    fn get_raw_bytes(self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}

impl Destenable for i8 {
    fn from_raw_bytes(data: Vec<u8>) -> Option<Self> {
        data.try_into().map(i8::from_le_bytes).ok()
    }
}

impl Stenable for i16 {
    fn get_raw_bytes(self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}

impl Destenable for i16 {
    fn from_raw_bytes(data: Vec<u8>) -> Option<Self> {
        data.try_into().map(i16::from_le_bytes).ok()
    }
}

impl Stenable for i32 {
    fn get_raw_bytes(self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}

impl Destenable for i32 {
    fn from_raw_bytes(data: Vec<u8>) -> Option<Self> {
        data.try_into().map(i32::from_le_bytes).ok()
    }
}

impl Stenable for i64 {
    fn get_raw_bytes(self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}

impl Destenable for i64 {
    fn from_raw_bytes(data: Vec<u8>) -> Option<Self> {
        data.try_into().map(i64::from_le_bytes).ok()
    }
}

impl Stenable for f32 {
    fn get_raw_bytes(self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}

impl Destenable for f32 {
    fn from_raw_bytes(data: Vec<u8>) -> Option<Self> {
        data.try_into().map(f32::from_le_bytes).ok()
    }
}

impl Stenable for f64 {
    fn get_raw_bytes(self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}

impl Destenable for f64 {
    fn from_raw_bytes(data: Vec<u8>) -> Option<Self> {
        data.try_into().map(f64::from_le_bytes).ok()
    }
}

impl<T: Stenable, E: Stenable> Stenable for Result<T, E> {
    fn get_raw_bytes(self) -> Vec<u8> {
        match self {
            Ok(t) => t.get_raw_bytes(),
            Err(e) => e.get_raw_bytes(),
        }
    }
}

impl<T: Destenable, E: Destenable> Destenable for Result<T, E> {
    fn from_raw_bytes(data: Vec<u8>) -> Option<Self> {
        T::from_raw_bytes(data.clone())
            .map(Ok)
            .or_else(|| E::from_raw_bytes(data).map(Err))
    }
}

pub mod prelude {
    pub use super::image_io::Image;
    pub use super::{Container, Destenable, StenError, Stenable};
}

#[cfg(test)]
mod tests {
    use super::*;
    type VecByte = Vec<u8>;
    macro_rules! make_test_sten {
        ($test_name: ident, $container_size: expr, $payload: expr, $payload_type: ty) => {
            #[test]
            fn $test_name() {
                let mut container = vec![13; $container_size];
                let payload = $payload;
                payload.sten(&mut container).unwrap();
                let new_payload = <$payload_type>::desten(&container).unwrap();
                assert_eq!(new_payload, $payload);
            }
        };
    }

    make_test_sten!(test_sten_single_byte, 256, vec![255], VecByte);
    make_test_sten!(test_sten_many_bytes, 256, vec![1, 3, 5, 7, 9], VecByte);
    make_test_sten!(test_sten_string, 256, String::from("Hello, Sten"), String);
    make_test_sten!(test_sten_u8, 256, 100u8, u8);
    make_test_sten!(test_sten_u16, 256, 1234u16, u16);
    make_test_sten!(test_sten_u32, 256, 0x89ABCDEFu32, u32);
    make_test_sten!(test_sten_u64, 256, 0x1234567890ABCDEFu64, u64);
    make_test_sten!(test_sten_i8, 256, 100i8, i8);
    make_test_sten!(test_sten_i16, 256, 1234i16, i16);
    make_test_sten!(test_sten_i32, 256, 0x19ABCDEFi32, i32);
    make_test_sten!(test_sten_i64, 256, 0x1234567890ABCDEFi64, i64);
    make_test_sten!(test_sten_f32, 256, 0.12345678f32, f32);
    make_test_sten!(test_sten_f64, 256, 0.1234567890123456f64, f64);
}
