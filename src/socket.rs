use fnv::FnvHashMap as HashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{spawn as spawn_thread, Thread, JoinHandle};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::net::{ToSocketAddrs, UdpSocket};
use std::time::Duration;
use std::ops::Deref;

type RemoteID = ();

#[derive(Debug)]
pub enum SocketMainThreadFatalError {}
#[derive(Debug)]
pub enum SocketRecvError {}

#[derive(Debug)]
pub struct InMsg(pub RemoteID, pub Arc<[u8]>);
pub type OutMsg = InMsg;

#[derive(Debug)]
pub struct Remote {
    id: RemoteID
}

#[derive(Debug)]
pub struct Socket {
    should_stop: Arc<AtomicBool>,
    thread_handle: JoinHandle<Result<(), SocketMainThreadFatalError>>,
    incoming_msg_receiver: Receiver<InMsg>,
    outgoing_msg_sender: Sender<OutMsg>
}

struct SocketMainThreadContext {
    pub udp_socket: UdpSocket,
    pub in_msg_sender: Sender<InMsg>,
    pub out_msg_receiver: Receiver<OutMsg>,
    pub should_stop: Arc<AtomicBool>,
}

impl SocketMainThreadContext {
    fn start(self) -> Result<(), SocketMainThreadFatalError> {
        let poll_interval = Duration::from_millis(10);
        while !self.should_stop.deref().load(Ordering::Relaxed) {
            // do stuff

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

fn spawn_socket_main_thread(udp_socket: UdpSocket, in_msg_sender: Sender<InMsg>, out_msg_receiver: Receiver<OutMsg>, should_stop: Arc<AtomicBool>)
     -> JoinHandle<Result<(), SocketMainThreadFatalError>> {
    let thread_builder = ::std::thread::Builder::new();
    thread_builder.name(String::from("socket_main_thread")).spawn(move || {
        SocketMainThreadContext {
            udp_socket,
            in_msg_sender,
            out_msg_receiver,
            should_stop,
        }.start()
    }).expect("Could not spawn socket_main_thread correctly")
}

impl Socket {
    fn new<A: ToSocketAddrs>(address: A) -> ::std::io::Result<Socket> {
        let udp_socket = UdpSocket::bind(address)?;
        let (in_msg_sender, in_msg_receiver) = channel::<InMsg>();
        let (out_msg_sender, out_msg_receiver) = channel::<OutMsg>();
        let should_stop = Arc::new(AtomicBool::new(false));

        let thread_handle = spawn_socket_main_thread(udp_socket, in_msg_sender, out_msg_receiver, should_stop.clone());

        Ok(Socket {
            should_stop,
            thread_handle,
            incoming_msg_receiver: in_msg_receiver,
            outgoing_msg_sender: out_msg_sender,
        })
    }

    fn shutdown(self) -> Result<(), SocketMainThreadFatalError> {
        self.should_stop.as_ref().store(true, Ordering::Relaxed);
        let r = self.thread_handle.join().unwrap();
        r
    }
}

#[test]
fn socket_init_destroy() {
    let socket = Socket::new("0.0.0.0:0").unwrap();
    ::std::thread::sleep(::std::time::Duration::from_millis(10));
    let handle = socket.shutdown().unwrap();
}