//! Wire representation of a player input. Kept here so `hg-sim` stays free of serde.

use hg_sim::PlayerInput;
use serde::{Deserialize, Serialize};

/// GGRS input type: the 16-bit command from [`hg_sim::PlayerInput`].
#[repr(transparent)]
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    bytemuck::Pod,
    bytemuck::Zeroable,
)]
pub struct NetInput(pub u16);

impl From<NetInput> for PlayerInput {
    fn from(v: NetInput) -> Self {
        PlayerInput(v.0).sanitized()
    }
}

/// GGRS address: the remote player's handle (0..4). The JS layer maps it to a `DataChannel`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PeerSlot(pub u8);
