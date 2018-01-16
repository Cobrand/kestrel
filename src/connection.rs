use fnv::FnvHashMap as HashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{spawn as spawn_thread, Thread, JoinHandle};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::net::{ToSocketAddrs, UdpSocket};
use std::time::Duration;
use std::ops::Deref;
use socket::RemoteID;

#[derive(Debug)]
pub enum ConnectionMainThreadFatalError {}
#[derive(Debug)]
pub enum ConnectionRecvError {}

#[derive(Debug)]
pub struct InMsg(pub RemoteID, pub Arc<[u8]>);
pub type OutMsg = InMsg;

#[derive(Debug)]
pub struct Connection {
    should_stop: Arc<AtomicBool>,
    thread_handle: JoinHandle<Result<(), ConnectionMainThreadFatalError>>,
    incoming_msg_receiver: Receiver<InMsg>,
    outgoing_msg_sender: Sender<OutMsg>
}

struct ConnectionMainThreadContext {
    pub udp_socket: UdpSocket,
    pub in_msg_sender: Sender<InMsg>,
    pub out_msg_receiver: Receiver<OutMsg>,
    pub should_stop: Arc<AtomicBool>,
}


impl ConnectionMainThreadContext {
    fn start(self) -> Result<(), ConnectionMainThreadFatalError> {
        let poll_interval = Duration::from_millis(10);
        let should_stop = self.should_stop.load(Ordering::Relaxed);

        while !should_stop {
            

            ::std::thread::sleep(poll_interval);
        }
        Ok(())
    }

    fn receive_incoming(&mut self) {
        unimplemented!()
    }

    fn send_outgoing(&mut self) {
        unimplemented!()
    }
}

fn spawn_connection_main_thread(udp_socket: UdpSocket, in_msg_sender: Sender<InMsg>, out_msg_receiver: Receiver<OutMsg>, should_stop: Arc<AtomicBool>)
     -> JoinHandle<Result<(), ConnectionMainThreadFatalError>> {
    let thread_builder = ::std::thread::Builder::new();
    thread_builder.name(String::from("connection_main_thread")).spawn(move || {
        ConnectionMainThreadContext {
            udp_socket,
            in_msg_sender,
            out_msg_receiver,
            should_stop,
        }.start()
    }).expect("Could not spawn connection_main_thread correctly")
}

impl Connection {
    fn new<A: ToSocketAddrs>(address: A) -> ::std::io::Result<Connection> {
        let udp_socket = UdpSocket::bind(address)?;
        let (in_msg_sender, in_msg_receiver) = channel::<InMsg>();
        let (out_msg_sender, out_msg_receiver) = channel::<OutMsg>();
        let should_stop = Arc::new(AtomicBool::new(false));

        let thread_handle = spawn_connection_main_thread(udp_socket, in_msg_sender, out_msg_receiver, should_stop.clone());

        Ok(Connection {
            should_stop,
            thread_handle,
            incoming_msg_receiver: in_msg_receiver,
            outgoing_msg_sender: out_msg_sender,
        })
    }

    fn shutdown(self) -> Result<(), ConnectionMainThreadFatalError> {
        self.should_stop.as_ref().store(true, Ordering::Relaxed);
        let r = self.thread_handle.join().unwrap();
        r
    }
}

#[test]
fn connection_init_destroy() {
    let connection = Connection::new("0.0.0.0:0").unwrap();
    ::std::thread::sleep(::std::time::Duration::from_millis(10));
    let _handle = connection.shutdown().unwrap();
}

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