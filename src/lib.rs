#![allow(dead_code)]
#![allow(unused_imports)]

extern crate fnv;

extern crate itertools;

extern crate crc;
extern crate byteorder;

#[macro_use]
extern crate failure;

mod misc;
mod consts;
mod connection;
mod fragment_combiner;
mod fragment;
mod udp_message;
mod socket;

pub use connection::{Connection, ConnectionMainThreadFatalError, ConnectionRecvError};
pub use socket::*;