#[cfg(test)]
mod tests {
    use std::net::UdpSocket;
    use super::super::consts::*;
    // Actually tests that the network is alright
    //
    // This acts as a point of reference to make there is are no nasty things such as permission
    // issues, networking issues or other pain in the neck like these while testing.
    // Basically, if every other test fails but this one works, first give yourself a pat in the back
    // because it must have been really hard to break all of this at once, and second, maybe it's your
    // brain that's wrong in this crate, not the test suite.
    #[test]
    fn prelude() {
        let mut buffer: [u8; MAX_UDP_MESSAGE_SIZE] = [0; MAX_UDP_MESSAGE_SIZE];
        let udp_receiver = UdpSocket::bind("127.0.0.1:51793").unwrap();
        let udp_sender = UdpSocket::bind("0.0.0.0:0").unwrap();
        udp_sender.send_to(&[0u8, 1, 2, 3], "127.0.0.1:51793").unwrap();
        let (_received_size, _) = udp_receiver.recv_from(&mut buffer).unwrap();
        assert_eq!(buffer[0..4], [0u8, 1, 2, 3]);
    }
}