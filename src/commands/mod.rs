//! Commands module for Robin CLI.
//!
//! This module contains all the subcommand implementations, including
//! batman-adv settings, translation tables, routing, and mesh interface management.
//! Each submodule provides both the `clap::Command` for CLI parsing
//! and helper functions for formatting or printing the output.

mod aggregation;
mod ap_isolation;
mod bridge_loop_avoidance;
mod gateway_events;
mod gateways;
mod gw_mode;
mod interface;
mod mesh;
mod neighbors;
mod originators;
mod routing_algo;
mod transglobal;
mod translocal;
mod utils;

pub(crate) use aggregation::*;
pub(crate) use ap_isolation::*;
pub(crate) use bridge_loop_avoidance::*;
pub(crate) use gateway_events::*;
pub(crate) use gateways::*;
pub(crate) use gw_mode::*;
pub(crate) use interface::*;
pub(crate) use mesh::*;
pub(crate) use neighbors::*;
pub(crate) use originators::*;
pub(crate) use routing_algo::*;
pub(crate) use transglobal::*;
pub(crate) use translocal::*;
pub(crate) use utils::*;
