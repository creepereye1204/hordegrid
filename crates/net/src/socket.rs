//! `NonBlockingSocket` adapter: GGRS talks to in-memory queues; the host (JS or a test harness)
//! moves bytes between those queues and real transports.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use ggrs::{Message, NonBlockingSocket};

use crate::codec;
use crate::input::PeerSlot;

/// Inbox cap: beyond this, packets are dropped (e.g. a tab returning from background).
pub const INBOX_CAP: usize = 1024;

/// Shared packet queues.
#[derive(Default, Debug)]
pub struct Queues {
    /// Received, not yet consumed by GGRS.
    pub inbox: VecDeque<(PeerSlot, Vec<u8>)>,
    /// Produced by GGRS, not yet sent by the host.
    pub outbox: Vec<(PeerSlot, Vec<u8>)>,
    /// Counters for diagnostics.
    pub stats: SocketStats,
}

/// Drop counters.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketStats {
    /// Decode failures.
    pub malformed: u32,
    /// Messages too large to send.
    pub oversize: u32,
    /// Inbox overflow drops.
    pub overflow: u32,
    /// Protocol version mismatches.
    pub version_mismatch: u32,
}

/// Handle shared by the session (as its socket) and the runner (to feed/drain packets).
pub type SharedQueues = Rc<RefCell<Queues>>;

/// The socket given to GGRS.
pub struct BridgeSocket {
    queues: SharedQueues,
}

impl BridgeSocket {
    /// Create a socket and the handle to its queues.
    pub fn new() -> (Self, SharedQueues) {
        let queues = SharedQueues::default();
        (
            Self {
                queues: Rc::clone(&queues),
            },
            queues,
        )
    }
}

impl Queues {
    /// Host → GGRS: enqueue a received datagram.
    pub fn push_incoming(&mut self, from: PeerSlot, bytes: &[u8]) {
        if self.inbox.len() >= INBOX_CAP {
            self.stats.overflow += 1;
            return;
        }
        self.inbox.push_back((from, bytes.to_vec()));
    }
}

impl NonBlockingSocket<PeerSlot> for BridgeSocket {
    fn send_to(&mut self, msg: &Message, addr: &PeerSlot) {
        let mut q = self.queues.borrow_mut();
        match codec::encode(msg) {
            Some(bytes) => q.outbox.push((*addr, bytes)),
            None => q.stats.oversize += 1,
        }
    }

    fn receive_all_messages(&mut self) -> Vec<(PeerSlot, Message)> {
        let mut q = self.queues.borrow_mut();
        let drained: Vec<_> = q.inbox.drain(..).collect();
        let mut out = Vec::with_capacity(drained.len());
        for (from, bytes) in drained {
            match codec::decode(&bytes) {
                Ok(msg) => out.push((from, msg)),
                Err(codec::DecodeError::Version(_)) => q.stats.version_mismatch += 1,
                Err(_) => q.stats.malformed += 1,
            }
        }
        out
    }
}
