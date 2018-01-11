
pub (crate) const CRC32_SIZE: usize = 4;

// 4 bytes for the seq_id, 1 for the frag_id, 1 for the
pub (crate) const FRAG_HEADER_SIZE: usize = 4 + 1 + 1;

// 1024 + 256 is an arbitrary value below most common MTU values
// since the baseline is around 1400, 1280 for the "inner" message + udp message header of 10 bytes
// is not too bad, although we could do better.
pub (crate) const MAX_UDP_MESSAGE_SIZE: usize = 1024 + 256 + CRC32_SIZE + FRAG_HEADER_SIZE;

// we limit the amount of fragments to 64 here, because we would like to code ack messages
// on 64bits (1 bit per fragment received), thus having only 1 message for 1 seq_id
// this *should* be enough for fast paced games, as you can send up to 81KB in 1 sequence
pub (crate) const MAX_FRAGMENTS_IN_MESSAGE: usize = 64;