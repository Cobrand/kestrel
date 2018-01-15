extern crate kestrel as k;

use std::net::UdpSocket;

fn main() {
    let udp_socket = UdpSocket::bind("0.0.0.0:50823").unwrap();
    let mut socket = k::Socket::new(&udp_socket);
    let remote_id = socket.try_connect("127.0.0.1:5514").unwrap();
    let buf = vec!(b'a'; 81921);
    socket.send_forgettable_message(remote_id, buf.as_slice(), 0).unwrap();
}