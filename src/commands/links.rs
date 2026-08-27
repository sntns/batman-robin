use crate::commands::{get_mesh_address, get_originators};
use crate::error::Error;
use crate::model::Link;

/// Retrieves the one-hop links of a BATMAN-adv mesh interface.
///
/// This reuses [`get_originators`], which already dumps every candidate route
/// per originator, and keeps only the entries where the originator was reached
/// directly (`next_hop == originator`). Per batman-adv protocol semantics, such
/// an entry was received with zero forwarding hops, i.e. it represents a
/// genuine direct radio link rather than a multi-hop route, and its `tq`/
/// `throughput` is the raw one-hop link quality.
///
/// # Arguments
///
/// * `ifindex` - The mesh interface index.
///
/// # Returns
///
/// Returns a vector of `Link` entries or a `Error` if the query fails.
pub async fn get_links(ifindex: u32) -> Result<Vec<Link>, Error> {
    let local = get_mesh_address(ifindex).await?;
    let origins = get_originators(ifindex).await?;

    Ok(origins
        .into_iter()
        .filter(|o| o.next_hop == o.originator)
        .map(|o| Link {
            local,
            neighbor: o.originator,
            outgoing_if: o.outgoing_if,
            last_seen_ms: o.last_seen_ms,
            tq: o.tq,
            throughput_kbps: o.throughput,
        })
        .collect())
}
