//! EXOline TCP client.
//! Reads data from a device.
//!
//! Start with the [EXOlineTCPClient].

mod client_impl;
mod exoline_exception;
mod internal;
mod variant;

pub use client_impl::{EXOlineError, EXOlineTCPClient};
pub use exoline_exception::EXOlineException;
pub use variant::Variant;
