//! Traffic Simulation Library
//!
//! A traffic simulation library that can run independently or with a Bevy UI.

pub mod simulation;

#[cfg(feature = "ui")]
pub mod ui;
