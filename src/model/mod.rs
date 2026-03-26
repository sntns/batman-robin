//! Data models and abstractions for Robin.
//!
//! This module defines the core types used for representing batman-adv
//! state, attributes, clients, gateways, interfaces, neighbors, originators,
//! translation tables, and utility functions.
//!
//! Each submodule focuses on a specific area of the mesh network model.

mod attribute;
mod client_flag;
mod command;
mod gateway;
mod gateway_event;
mod interface;
mod interface_selector;
mod mesh;
mod neighbor;
mod originator;
mod transtable;
mod utils;

pub use attribute::*;
pub use client_flag::*;
pub use command::*;
pub use gateway::*;
pub use gateway_event::*;
pub use interface::*;
pub use interface_selector::*;
pub use mesh::*;
pub use neighbor::*;
pub use originator::*;
pub use transtable::*;
pub use utils::*;
