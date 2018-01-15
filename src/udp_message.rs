use byteorder::{BigEndian, ByteOrder};
use consts::*;
use fragment::*;
use misc::*;

use crc::crc32::checksum_ieee as crc32_check;

#[derive(Debug)]
pub (crate) struct UdpMessage<B: AsRef<[u8]>> {
   pub (self) buffer: B
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

impl<'a, T: AsRef<[u8]>> From<&'a Fragment<T>> for UdpMessage<Box<[u8]>> {
    fn from(f: &'a Fragment<T>) -> UdpMessage<Box<[u8]>> {
        let mut bytes_mut: Vec<u8> = vec!(0u8; CRC32_SIZE + FRAG_HEADER_SIZE + f.data.as_ref().len());
        BigEndian::write_u32(&mut bytes_mut[4..8], f.seq_id);
        // write frag_id and frag_total as u8s
        bytes_mut[8] = f.frag_id;
        bytes_mut[9] = f.frag_total;
        (&mut bytes_mut[10..]).copy_from_slice(f.data.as_ref());
        let generated_crc: u32 = crc32_check(&bytes_mut[4..]);
        BigEndian::write_u32(&mut bytes_mut[0..4], generated_crc);
        UdpMessage {buffer: bytes_mut.into_boxed_slice()}
    }
}

impl<B: AsRef<[u8]>> UdpMessage<B> {
    fn check_header(udp_message: &[u8]) -> Result<(u32, u8, u8), UdpMessageError> {
        let buffer = udp_message;
        if buffer.len() < 10 {
            return Err(UdpMessageError::NotBigEnough);
        }
        let message_crc32: u32 = BigEndian::read_u32(&buffer[0..4]);
        let seq_id: u32 = BigEndian::read_u32(&buffer[4..8]);
        let frag_id: u8 = buffer[8];
        let frag_total: u8 = buffer[9];
        if frag_total >= 64 {
            return Err(UdpMessageError::FragTotalTooLarge)
        }
        let computed_crc32 = crc32_check(&buffer[4..]);
        if computed_crc32 != message_crc32 {
            return Err(UdpMessageError::InvalidCrc)
        }
        // since frag_total is really +1, if frag_id == frag_total, it's actually the last fragment
        // that we received. if frag_id = frag_total = 0, the first and last fragment of a message was received.
        if frag_id > frag_total {
            return Err(UdpMessageError::InvalidFragInfo)
        }
        Ok((seq_id, frag_id, frag_total))
    }

    pub (crate) fn new(b: B) -> UdpMessage<B>{
        UdpMessage {buffer: b}
    }

    /// Reads one message from a udp socket and returns its content as a UdpMessage
    ///
    /// Proper parameters that you see fit must have been set on UdpSocket. For instance,
    /// it may be wise to set this udp socket as non-blocking  if you don't want to block
    /// your thread forever trying to read one message.
    pub fn from_udp_socket(udp_socket: &::std::net::UdpSocket) -> ::std::io::Result<(UdpMessage<Box<[u8]>>, ::std::net::SocketAddr)> {
        let mut buffer = vec!(0; MAX_UDP_MESSAGE_SIZE);
        let (message_size, socket_addr) = udp_socket.recv_from(buffer.as_mut_slice())?;
        buffer.truncate(message_size);
        let udp_message = UdpMessage {buffer: buffer.into_boxed_slice()};
        Ok((udp_message, socket_addr))
    }

    /// useful for debug purposes
    pub (crate) fn as_bytes(&self) -> &[u8] {
        self.buffer.as_ref()
    }
    
}
    
impl<'a> UdpMessage<&'a [u8]> {
    pub (crate) fn into_fragment(self) -> Result<Fragment<&'a [u8]>, UdpMessageError> {
        let (seq_id, frag_id, frag_total) = Self::check_header(self.buffer.as_ref())?;
        Ok(Fragment {
            seq_id,
            frag_id,
            frag_total,
            data: &self.buffer[10..]
        })
    }
}

impl UdpMessage<Box<[u8]>> {

    /// Tries to build a Fragment from a UdpMessage.
    ///
    ///  No copies of data are involved
    pub (crate) fn into_fragment(self) -> Result<Fragment<StrippedBoxedSlice<u8>>, UdpMessageError> {
        let (seq_id, frag_id, frag_total) = Self::check_header(self.buffer.as_ref())?;
        Ok(Fragment {
            seq_id,
            frag_id,
            frag_total,
            data: StrippedBoxedSlice::new(self.buffer, 10usize)
        })
    }
}