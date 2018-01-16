use std::net::UdpSocket;
use std::net::{ToSocketAddrs, SocketAddr};
use std::rc::Rc;
use std::cell::{Cell, UnsafeCell};
use fnv::FnvHashMap as HashMap;
use failure::Fail;
use std::ops::Deref;
use std::collections::VecDeque;

use std::io::{Error, ErrorKind};

use consts::*;
use misc::*;
use udp_message::*;
use fragment::*;
use fragment_combiner::*;

pub type RemoteID = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteStatus {
    /// No connection attempt has been made yet
    NotStarted,
    /// Trying to connect to remote.
    ///
    /// Parameter is the
    /// number of loops we've had internally without response.
    ///
    /// If this parameter reaches the equivalent of 10 seconds,
    /// the connection becomes disconnected
    Connecting(u32),
    /// Connection request from remote accepted, waiting
    /// for connection acknowlegment
    ///
    /// See Connecting(_) for an explanation on the parameter
    AckConnecting(u32),
    /// Connected to remote
    Connected,
    /// Disconnected from remote. Remote may be destroyed anytime soon
    Disconnected,
}

impl Default for RemoteStatus {
    /// A remote's default status is NotStarted
    fn default() -> Self {
        RemoteStatus::NotStarted
    }
}

#[derive(Debug)]
struct Remote {
    pub (self) id: RemoteID,
    pub (self) remote_socket_addr: SocketAddr,
    pub (self) status: Cell<RemoteStatus>,
    pub (self) next_seq_id: Cell<u32>,
    fragment_combiner: UnsafeCell<FragmentCombiner<StrippedBoxedSlice<u8>>>,
}

impl Remote {
    pub fn push_udp_message(&self, udp_message: UdpMessage<Box<[u8]>>) {
        unsafe {
            let ref mut fragment_combiner = *self.fragment_combiner.get();
            match udp_message.into_fragment() {
                Ok(fragment) => {
                    fragment_combiner.push(fragment);
                },
                Err(_) => {
                    // TODO handle the error
                    // maybe log something?
                }
            }
        }
    }

    /// calls FragmentCombiner::extract_out_messages
    pub fn extract_out_messages(&self) -> VecDeque<Box<[u8]>> {
        unsafe {
            let fragment_combiner_ptr = self.fragment_combiner.get();
            let ref mut fragment_combiner = *fragment_combiner_ptr;
            fragment_combiner.extract_out_messages()
        }
    }
}


#[derive(Debug, Copy, Clone)] 
pub enum MessageType {
    /// Forgettable message type.
    ///
    /// If the message did not make
    /// it through the end the first time, abandon this message.
    Forgettable,
    /// A droppable message type, that can be discarded if the network
    /// on the other side is suspected to be congested.
    ///
    /// Like Forgettable, if a message of this type doesn't make it
    /// the first time, it will be discarded.
    Droppable,
    /// A Key but expirable message.
    ///
    /// The parameter holds the number of
    /// milliseconds this message expires after. If this parameter is 0,
    /// the behavior is the same as Forgettable.
    ///
    /// As long as this message is still valid, it will try to re-send
    /// messages if Socket suspects it did not get the message in time.
    KeyExpirableMessage(u32),
    /// A key message that should arrive everytime.
    ///
    /// A long at the socket doesn't receive the correct ack for this message,
    /// this message will be re-sent.
    KeyMessage,
}

#[derive(Debug, Fail)]
pub enum SocketError {
    #[fail(display = "Invalid Remote ID: {:?}", _0)]
    InvalidRemoteId(RemoteID),
    #[fail(display = "IO error: {}", _0)]
    IoError(::std::io::Error),
}

impl From<::std::io::Error> for SocketError {
    fn from(e: ::std::io::Error) -> SocketError {
        SocketError::IoError(e)
    }
}

#[derive(Debug)]
pub struct Socket {
    next_remote_id: RemoteID,
    udp_socket: UdpSocket,
    remotes: HashMap<RemoteID, Rc<Remote>>,
    remotes_by_addr: HashMap<SocketAddr, Rc<Remote>>,
}

impl Socket {
    pub fn new(udp_socket: UdpSocket) -> Socket {
        udp_socket.set_nonblocking(true).unwrap();
        Socket {
            next_remote_id: 0,
            udp_socket,
            remotes: Default::default(),
            remotes_by_addr: Default::default(),
        }
    }

    pub fn try_connect<A: ToSocketAddrs>(&mut self, remote_addr: A) -> ::std::io::Result<RemoteID> {
        let remote_addr = remote_addr.to_socket_addrs()?.next().unwrap();
        let remote = Remote {
            id: self.next_remote_id,
            remote_socket_addr: remote_addr,
            status: Default::default(),
            next_seq_id: Cell::new(0),
            fragment_combiner: UnsafeCell::new(FragmentCombiner::new()),
        };
        // TODO send a message here
        // TODO change the status here
        
        let remote_id = self.next_remote_id;
        let remote = Rc::new(remote);
        self.remotes.insert(remote_id, remote.clone());
        self.remotes_by_addr.insert(remote_addr, remote);

        self.next_remote_id += 1;
        Ok(remote_id)
    }

    pub fn prepare_iteration(&mut self) {
        let mut done = false;
        while !done {
            match UdpMessage::<Box<[u8]>>::from_udp_socket(&self.udp_socket) {
                Ok((udp_message, socket_addr)) => {
                    let remote = self.remotes_by_addr.get(&socket_addr);
                    match remote {
                        None => {
                            // maybe it's someone who tries to connect? Let's try to make contact
                            unimplemented!()
                        },
                        Some(ref remote) => {
                            // remote is valid, let's push the message into this remote
                            remote.push_udp_message(udp_message);
                        }
                    }
                },
                Err(e) => {
                    match e.kind() {
                        ErrorKind::WouldBlock => {done = true},
                        e => {
                            panic!("Prepare iteration failed when receiving udp message: {:?}", e);
                        }
                    }
                }
            }
        }
    }

    // TODO when impl Trait is done, replace VecDeque by impl Trait
    /// Returns all received messages for a remote_id. The messages are in the order they *arrived*,
    /// but it may be different from the order the messages were *sent* from remote.
    ///
    /// You *must* call `prepare_iteration` right before calling this function if you want to receive the messages
    /// properly; otherwise incoming messages will be kept in the queue and you will have no way to have access
    /// to the new messages.
    pub fn receive_all_messages_from(&mut self, remote_id: RemoteID) -> Result<VecDeque<Box<[u8]>>, SocketError> {
        let remote = self.remotes.get(&remote_id).ok_or(SocketError::InvalidRemoteId(remote_id))?;
        Ok(remote.extract_out_messages())
    }

    /// Returns all received messages from all remotes
    ///
    /// You don't have to call `prepare_iteration`, it is automatically being done here.
    pub fn receive_all_messages(&mut self) -> Vec<(RemoteID, VecDeque<Box<[u8]>>)> {
        self.prepare_iteration();
        self.remotes
            .iter()
            .map(|(remote_id, remote)| {
                (*remote_id, remote.extract_out_messages())
            })
            .collect()
    }

    pub fn send_message(&mut self, remote_id: RemoteID, message: &[u8], t: MessageType, priority: i8) -> Result<(), SocketError> {
        let remote = self.remotes.get(&remote_id).ok_or(SocketError::InvalidRemoteId(remote_id))?;
        let seq_id = remote.deref().next_seq_id.get();
        let fragments = build_fragments_from_data(&message, seq_id).expect("TODO");
        for fragment in fragments {
            let udp_message = UdpMessage::from(&fragment);
            // TODO remove unwrap
            let _r = self.udp_socket.send_to(udp_message.as_bytes(), remote.remote_socket_addr).unwrap();
            // TODO log the error if any
        }
        remote.deref().next_seq_id.set(seq_id + 1);
        Ok(())
    }

    #[inline]
    pub fn send_key_message(&mut self, remote_id: RemoteID, message: &[u8], priority: i8) -> Result<(), SocketError> {
        self.send_message(remote_id, message, MessageType::KeyMessage, priority)
    }
    
    #[inline]
    pub fn send_key_expirable_message(&mut self, remote_id: RemoteID, message: &[u8], expiration_ms: u32, priority: i8) -> Result<(), SocketError> {
        self.send_message(remote_id, message, MessageType::KeyExpirableMessage(expiration_ms), priority)
    }

    #[inline]
    pub fn send_forgettable_message(&mut self, remote_id: RemoteID, message: &[u8], priority: i8) -> Result<(), SocketError> {
        self.send_message(remote_id, message, MessageType::Forgettable, priority)
    }

    #[inline]
    pub fn send_droppable_message(&mut self, remote_id: RemoteID, message: &[u8], priority: i8) -> Result<(), SocketError> {
        self.send_message(remote_id, message, MessageType::Droppable, priority)
    }
}