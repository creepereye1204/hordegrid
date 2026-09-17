//! GGRS glue: config, wire codec, socket bridge and session runners.
//!
//! Design: docs/design-docs/netcode-rollback.md. No JS/web-sys here — `hg-web` adapts to the browser.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

pub mod codec;
pub mod input;
pub mod session;
pub mod socket;

pub use input::{NetInput, PeerSlot};
pub use session::{LocalRunner, NetEvent, NetSettings, Runner, TickStatus};
