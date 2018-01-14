use bytes::{BytesMut, BufMut, ByteOrder, BigEndian};
use consts::*;
use fragment::*;

use crc::crc32::checksum_ieee as crc32_check;

#[derive(Debug)]
pub struct UdpMessage {
   buffer: BytesMut
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum UdpMessageError {
    /// Received data was not big enough to be a fragment
    NotBigEnough, // (That's what she said)
    /// The Crc inside the message was not valid
    InvalidCrc,
    /// Invalid Frag Info happens when frag_total + 1 is lower than frag_id
    InvalidFragInfo,
    /// Frag Total is too large (should be <= 63)
    FragTotalTooLarge,
}

impl<'a, T: AsRef<[u8]>> From<&'a Fragment<T>> for UdpMessage {
    fn from(f: &'a Fragment<T>) -> UdpMessage {
        let mut bytes_mut = BytesMut::with_capacity(CRC32_SIZE + FRAG_HEADER_SIZE + f.data.as_ref().len());
        // reserve 4 bytes for the crc32 later
        unsafe {bytes_mut.advance_mut(4)};
        bytes_mut.put_u32::<BigEndian>(f.seq_id);
        bytes_mut.put(f.frag_id);
        bytes_mut.put(f.frag_total);
        bytes_mut.put_slice(f.data.as_ref());
        let generated_crc: u32 = crc32_check(&bytes_mut[4..]);
        BigEndian::write_u32(&mut bytes_mut[0..4], generated_crc);
        UdpMessage {buffer: bytes_mut}
    }
}

/// Converts a slice into an UdpMessage. Copies the content
///
/// Note that the check is actually done at the *end* when a Fragment
/// is about to be created
impl<'a> From<&'a [u8]> for UdpMessage {
    fn from(s: &'a [u8]) -> UdpMessage {
        let mut bytes_mut = BytesMut::with_capacity(s.len());
        bytes_mut.put_slice(s);
        UdpMessage { buffer: bytes_mut }
    }
}

impl UdpMessage {
    /// Creates an allocated message for Udp Reading.
    pub fn new() -> UdpMessage {
        UdpMessage {buffer: BytesMut::with_capacity(MAX_UDP_MESSAGE_SIZE)}
    }

    /// Tries to build a Fragment from a UdpMessage.
    ///
    /// There are no copies involved, so the Fragment has a lifetime that depends
    /// on the UdpMessage. To have Boxed data, use `get_boxed_fragment`
    pub fn get_fragment<'a>(&'a self) -> Result<Fragment<&'a [u8]>, UdpMessageError> {
        if self.buffer.len() < 10 {
            return Err(UdpMessageError::NotBigEnough);
        }
        let message_crc32: u32 = BigEndian::read_u32(&self.buffer[0..4]);
        let seq_id: u32 = BigEndian::read_u32(&self.buffer[4..8]);
        let frag_id: u8 = self.buffer[8];
        let frag_total: u8 = self.buffer[9];
        if frag_total >= 64 {
            return Err(UdpMessageError::FragTotalTooLarge)
        }
        let computed_crc32 = crc32_check(&self.buffer[4..]);
        if computed_crc32 != message_crc32 {
            return Err(UdpMessageError::InvalidCrc)
        }
        // since frag_total is really +1, if frag_id == frag_total, it's actually the last fragment
        // that we received. if frag_id = frag_total = 0, the first and last fragment of a message was received.
        if frag_id > frag_total {
            return Err(UdpMessageError::InvalidFragInfo)
        }
        Ok(Fragment {
            seq_id,
            frag_id,
            frag_total,
            data: &self.buffer[10..]
        })
    }

    /// Tries to build an independant fragment from a UdpMessage
    ///
    /// The content of the UdpMessage will be copied into a Box.
    pub fn get_boxed_fragment(&self) -> Result<Fragment<Box<[u8]>>, UdpMessageError> {
        self.get_fragment().map(Fragment::into_boxed)
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.buffer.as_ref()
    }
}