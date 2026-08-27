use macaddr::MacAddr6;

/// Represents an originator node in the batman-adv mesh network.
///
/// An originator is a node that advertises itself or forwards packets for other nodes.
/// This struct contains information about the originator's MAC address, routing metrics,
/// and the interface used to reach it.
#[derive(Debug, Clone)]
pub struct Originator {
    /// MAC address of the originator node.
    /// Corresponds to `BATADV_ATTR_ORIG_ADDRESS`.
    pub originator: MacAddr6,

    /// MAC address of the next hop towards the originator.
    /// Corresponds to `BATADV_ATTR_NEIGH_ADDRESS`.
    pub next_hop: MacAddr6,

    /// Outgoing interface name or index used to reach the originator.
    /// Corresponds to `BATADV_ATTR_HARD_IFNAME` (or the interface index).
    pub outgoing_if: String,

    /// Time since the originator was last seen, in milliseconds.
    /// Corresponds to `BATADV_ATTR_LAST_SEEN_MSECS`.
    pub last_seen_ms: u32,

    /// Optional TQ (link quality) metric towards this originator.
    /// Corresponds to `BATADV_ATTR_TQ`.
    pub tq: Option<u8>,

    /// Optional throughput value towards this originator.
    /// Corresponds to `BATADV_ATTR_THROUGHPUT`.
    pub throughput: Option<u32>,

    /// Indicates whether this originator is considered the best next-hop router.
    /// Corresponds to `BATADV_ATTR_FLAG_BEST`.
    pub is_best: bool,
}
