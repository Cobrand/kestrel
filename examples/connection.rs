extern crate kestrel as k;

use std::time::Duration;
use std::thread::sleep;

use k::*;

fn main() {
    let mut connection1 = Connection::new("0.0.0.0:5212").unwrap();
    let mut connection2 = Connection::new("0.0.0.0:5213").unwrap();

    connection1.try_connect("127.0.0.1:5213").unwrap();
    connection2.try_connect("127.0.0.1:5212").unwrap();

    sleep(Duration::from_millis(10));
    let mut connection1_remote_id: Option<RemoteID> = None;
    let mut connection2_remote_id: Option<RemoteID> = None;
    
    match connection1.receive_event().unwrap().unwrap() {
        InEvent::NewConnectionFrom(_, remote_id, _) => {
            connection1_remote_id = Some(remote_id);
        },
        _ => {}
    }
    match connection2.receive_event().unwrap().unwrap() {
        InEvent::NewConnectionFrom(_, remote_id, _) => {
            connection2_remote_id = Some(remote_id);
        },
        _ => {}
    }

    
    let data1 = vec!(5u8);
    connection1.send_forgettable_data(connection1_remote_id.unwrap(), data1);
    let data2 = vec!(4u8);
    connection2.send_forgettable_data(connection2_remote_id.unwrap(), data2);
    let data3 = vec!(3u8);
    connection2.send_forgettable_data(connection2_remote_id.unwrap(), data3);

    sleep(Duration::from_millis(50));

    println!("{:?}", connection1.receive_data().unwrap().unwrap());
    println!("{:?}", connection1.receive_data().unwrap().unwrap());
    println!("{:?}", connection2.receive_data().unwrap().unwrap());
}