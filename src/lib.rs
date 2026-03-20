//! # Batman-Robin Library
//!
//! This crate provides a Rust interface to **batman-adv** mesh networking, exposing
//! both low-level netlink operations and a high-level `RobinClient` for CLI or programmatic use.
//!
//! ## Modules
//!
//! - `commands` - Internal implementation of batman-adv commands (netlink message builders, parsing, etc.).
//! - `error` - Defines `RobinError`, the unified error type for all operations.
//! - `netlink` - Low-level wrappers around netlink sockets, generic netlink messages, and attribute builders.
//! - `client` - High-level API providing the `RobinClient` struct for interacting with mesh networks.
//! - `model` - Data structures representing interfaces, neighbors, originators, gateways, translation tables, etc.
//! - `cli` - Command-line interface modules (only included when building the binary).

mod commands;
mod error;
mod netlink;

pub mod cli;
pub mod client;
pub mod model;

pub use client::Client;
pub use error::RobinError;
pub use model::*;
