//! hordegrid deterministic simulation.
//!
//! Contract (docs/design-docs/determinism.md): integer/fixed-point only, no time, no hash-order,
//! state is `Pod` with no padding, and `World::step` is the only mutator.

#![forbid(unsafe_code)]
#![deny(
    clippy::float_arithmetic,
    clippy::disallowed_types,
    clippy::unwrap_used
)]

pub mod config;
pub mod flow;
pub mod fx;
pub mod grid;
pub mod hide_seek;
pub mod input;
pub mod map;
pub mod physics;
pub mod rng;
pub mod state;
pub mod step;

pub use fx::Fixed;
pub use input::PlayerInput;
pub use state::{GameMode, State};
pub use step::World;
