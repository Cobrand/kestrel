use fnv::FnvHashMap as HashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{spawn as spawn_thread, Thread, JoinHandle};
use std::sync::mpsc::{Receiver, Sender, channel, TryRecvError};
use std::net::{UdpSocket, ToSocketAddrs, SocketAddr};
use std::time::Duration;
use std::ops::Deref;

use socket::{RemoteID, Socket, MessageType};

#[derive(Debug)]
pub enum ConnectionMainThreadFatalError {}
#[derive(Debug)]
pub enum ConnectionRecvError {}

#[derive(Debug)]
pub struct InData(pub RemoteID, pub Box<[u8]>);

#[derive(Debug)]
pub struct OutData<B: AsRef<[u8]> + Sync + Send> {
    pub remote_id: RemoteID,
    pub data: B,
    pub priority: i8,
    pub message_type: MessageType
}

#[derive(Debug, Clone, Copy)]
pub enum InEvent {
    /// bool means "initiated by remote", so true if it
    /// was intiated by remote, false if we made the request ourselves
    NewConnectionFrom(SocketAddr, RemoteID, bool),
    /// RemoteID was disconnected
    Disconnected(RemoteID)
}

#[derive(Debug, Clone, Copy)]
pub enum OutEvent {
    NewConnection(SocketAddr),
    Disconnect(RemoteID),
}

#[derive(Debug)]
pub struct Connection<O: AsRef<[u8]> + Sync + Send> {
    should_stop: Arc<AtomicBool>,
    thread_handle: JoinHandle<Result<(), ConnectionMainThreadFatalError>>,
    incoming_data_receiver: Receiver<InData>,
    incoming_event_receiver: Receiver<InEvent>,
    outgoing_data_sender: Sender<OutData<O>>,
    outgoing_event_sender: Sender<OutEvent>,
}

struct ConnectionThreadContext<O: AsRef<[u8]> + Sync + Send> {
    pub socket: Socket,
    pub in_data_sender: Sender<InData>,
    pub in_event_sender: Sender<InEvent>,
    pub out_data_receiver: Receiver<OutData<O>>,
    pub out_event_receiver: Receiver<OutEvent>,
    pub should_stop: Arc<AtomicBool>,
}


impl<O: AsRef<[u8]> + Sync + Send> ConnectionThreadContext<O> {
    fn send_event_to_main(&self, event: InEvent) { 
        let r = self.in_event_sender.send(event);
        if let Err(_) = r {
            self.shutdown();
        }
    }

    fn start(mut self) -> Result<(), ConnectionMainThreadFatalError> {
        let poll_interval = Duration::from_millis(10);
        while !self.should_stop.load(Ordering::Relaxed) {
            self.process_outgoing_events();
            self.send_outgoing();
            self.receive_incoming();
            ::std::thread::sleep(poll_interval);
        }
        Ok(())
    }

    fn process_outgoing_events(&mut self) {
        loop {
            match self.out_event_receiver.try_recv() {
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.shutdown();
                    break;
                },
                Ok(OutEvent::NewConnection(socket_addr)) => {
                    let r = self.socket.try_connect(socket_addr);
                    match r {
                        Ok(remote_id) => self.send_event_to_main(InEvent::NewConnectionFrom(socket_addr, remote_id, false)),
                        Err(e) => println!("process_outgoing_events: NewConnection got error {:?}", e),
                    }
                },
                Ok(OutEvent::Disconnect(remote_id)) => {
                    unimplemented!()
                }
            }
        }
    }

    fn receive_incoming(&mut self) {
        'all_remotes: for (remote_id, remote_messages) in self.socket.receive_all_messages() {
            for message in remote_messages {
                let r = self.in_data_sender.send(InData(remote_id.clone(), message));

                if let Err(_) = r {
                    // if there is an error while sending the messages to the main thread,
                    // there's not point in continuing. It probably means that the main thread panicked
                    // somehow. TODO: log?
                    self.shutdown();
                    break 'all_remotes;
                }
            }
        }
    }

    fn send_outgoing(&mut self) {
        loop {
            match self.out_data_receiver.try_recv() {
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.shutdown();
                    break;
                },
                Ok(OutData { remote_id, data, priority, message_type}) => {
                    let r = self.socket.send_message(remote_id, data.as_ref(), message_type, priority);
                    if let Err(e) = r {
                        println!("Error while sendng message to {}: {:?}", remote_id, e);
                    }
                }
            }
        }
    }

    fn shutdown(&self) {
        self.should_stop.store(true, Ordering::Relaxed);
    }
}

impl<O: AsRef<[u8]> + Sync + Send + 'static> Connection<O> {
    /// Binds a connection to address `address`
    pub fn new<A: ToSocketAddrs>(address: A) -> ::std::io::Result<Connection<O>> {
        let udp_socket = UdpSocket::bind(address)?;
        let (in_data_sender, in_data_receiver) = channel::<InData>();
        let (in_event_sender, in_event_receiver) = channel::<InEvent>();
        let (out_data_sender, out_data_receiver) = channel::<OutData<O>>();
        let (out_event_sender, out_event_receiver) = channel::<OutEvent>();
        let should_stop = Arc::new(AtomicBool::new(false));

        let thread_handle = {
            let should_stop = should_stop.clone();
            let thread_builder = ::std::thread::Builder::new();
            thread_builder.name(String::from("connection_main_thread")).spawn(move || {
                ConnectionThreadContext {
                    socket: Socket::new(udp_socket),
                    in_data_sender,
                    in_event_sender,
                    out_data_receiver,
                    out_event_receiver,
                    should_stop,
                }.start()
            }).expect("Could not spawn connection_main_thread correctly")
        };

        Ok(Connection {
            should_stop,
            thread_handle,
            incoming_data_receiver: in_data_receiver,
            incoming_event_receiver: in_event_receiver,
            outgoing_data_sender: out_data_sender,
            outgoing_event_sender: out_event_sender,
        })
    }

    /// Stops the remote thread from running.
    pub fn shutdown(self) -> Result<(), ConnectionMainThreadFatalError> {
        // TODO disconnect the remote thread's remote 
        self.should_stop.as_ref().store(true, Ordering::Relaxed);
        let r = self.thread_handle.join().unwrap();
        r
    }

    pub fn send_data(&mut self, remote_id: RemoteID, data: O, message_type: MessageType, priority: i8) {
        self.outgoing_data_sender.send(OutData {
            remote_id,
            data,
            message_type,
            priority
        }).expect("could not connect to remote thread")
    }
    
    pub fn send_forgettable_data(&mut self, remote_id: RemoteID, data: O) {
        self.outgoing_data_sender.send(OutData {
            remote_id,
            data,
            message_type: MessageType::Forgettable,
            priority: 0
        }).expect("could not connect to remote thread")
    }

    pub fn receive_data(&mut self) -> Result<Option<InData>, ()> {
        match self.incoming_data_receiver.try_recv() {
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(()),
            Ok(m) => Ok(Some(m))
        }
    }
    
    pub fn receive_event(&mut self) -> Result<Option<InEvent>, ()> {
        match self.incoming_event_receiver.try_recv() {
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(()),
            Ok(m) => Ok(Some(m))
        }
    }

    /// Exec a request
    pub fn send_request(&mut self, event: OutEvent) {
        self.outgoing_event_sender.send(event).expect("could not connect to remote thread");
    }

    // Returns Error when the remote thread was killed
    pub fn try_connect<A: ToSocketAddrs>(&mut self, addr: A) -> Result<(), ()> {
        let socket_addr = addr.to_socket_addrs().unwrap().next().unwrap();
        let r = self.outgoing_event_sender.send(OutEvent::NewConnection(socket_addr));
        match r {
            Err(_) => Err(()),
            Ok(_) => Ok(())
        }
    }
}

#[test]
fn connection_init_destroy() {
    let connection = Connection::<Box<[u8]>>::new("0.0.0.0:0").unwrap();
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