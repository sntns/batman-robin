use macaddr::MacAddr6;
use serde::Serialize;

/// Represents a single one-hop link in the batman-adv mesh network.
///
/// Unlike [`crate::model::Originator`], which may describe a multi-hop route,
/// a `Link` always represents a direct radio link between this node and one of
/// its physical neighbors — i.e. an originator entry where the next hop is the
/// originator itself. This is the raw per-link quality data a mesh-wide
/// link-quality graph is built from.
#[derive(Debug, Clone, Serialize)]
pub struct Link {
    /// This node's own mesh MAC address (`BATADV_ATTR_MESH_ADDRESS`).
    pub local: MacAddr6,

    /// MAC address of the directly reachable neighbor.
    /// Corresponds to `BATADV_ATTR_ORIG_ADDRESS` / `BATADV_ATTR_NEIGH_ADDRESS`.
    pub neighbor: MacAddr6,

    /// Outgoing interface name used to reach the neighbor.
    /// Corresponds to `BATADV_ATTR_HARD_IFNAME` (or the interface index).
    pub outgoing_if: String,

    /// Time since the link was last seen, in milliseconds.
    /// Corresponds to `BATADV_ATTR_LAST_SEEN_MSECS`.
    pub last_seen_ms: u32,

    /// Optional TQ (link quality) metric of this link.
    /// Corresponds to `BATADV_ATTR_TQ`. Only available in BATMAN_IV mode.
    pub tq: Option<u8>,

    /// Optional throughput of this link in kilobits per second.
    /// Corresponds to `BATADV_ATTR_THROUGHPUT`. Only available in BATMAN_V mode.
    pub throughput_kbps: Option<u32>,
}
