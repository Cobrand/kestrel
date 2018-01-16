extern crate kestrel as k;

use std::net::UdpSocket;
use std::time::Duration;

fn main() {
    let poll_interval = Duration::from_millis(50);

    let udp_socket1 = UdpSocket::bind("0.0.0.0:50823").unwrap();
    let mut socket1 = k::Socket::new(&udp_socket1);
    let udp_socket2 = UdpSocket::bind("0.0.0.0:50824").unwrap();
    let mut socket2 = k::Socket::new(&udp_socket2);
    let socket1_remote_id = socket1.try_connect("127.0.0.1:50824").unwrap();
    let socket2_remote_id = socket2.try_connect("127.0.0.1:50823").unwrap();
    let buf = vec!(0u8; 1);
    socket1.send_forgettable_message(socket1_remote_id, buf.as_slice(), 0).unwrap();
    socket1.send_forgettable_message(socket1_remote_id, buf.as_slice(), 0).unwrap();
    socket1.send_forgettable_message(socket1_remote_id, buf.as_slice(), 0).unwrap();
    socket1.send_forgettable_message(socket1_remote_id, buf.as_slice(), 0).unwrap();
    for i in 0..10 {
        socket2.prepare_iteration();
        match socket2.receive_all_messages_from(socket2_remote_id) {
            Ok(o) => {
                println!("socket received messages: {:?}", o);
            },
            Err(e) => {
                println!("socket2 had an error: {:?}", e);
            }
        }
    };
}