use crate::commands::if_indextoname;
use crate::error::Error;
use crate::model::{AttrValueForSend, Attribute, Command, Originator};
use crate::netlink;

use macaddr::MacAddr6;
use neli::consts::nl::NlmF;
use neli::consts::nl::Nlmsg;
use neli::genl::Genlmsghdr;
use neli::nl::NlPayload;
use neli::nl::Nlmsghdr;

/// Retrieves the list of originators for a BATMAN-adv mesh interface.
///
/// This corresponds to the `batctl o` command. Each originator entry includes
/// the originator's MAC address, the next-hop neighbor MAC, the outgoing interface,
/// the last seen timestamp in milliseconds, optional TQ (link quality) and throughput,
/// and a flag indicating if this originator is currently the best route.
///
/// # Arguments
///
/// * `ifindex` - The mesh interface index.
///
/// # Returns
///
/// Returns a vector of `Originator` structs or a `Error` if the query fails.
///
/// # Example
///
/// ```no_run
/// # use batman_robin::model::Originator;
/// # async fn example() {
/// # let originators: Vec<Originator> = vec![];
/// // let originators = get_originators("bat0").await?;
/// for o in originators {
///     println!(
///         "Originator {} via {} (last seen {} ms, best: {})",
///         o.originator, o.outgoing_if, o.last_seen_ms, o.is_best
///     );
/// }
/// # }
/// ```
pub async fn get_originators(ifindex: u32) -> Result<Vec<Originator>, Error> {
    let mut attrs = netlink::GenlAttrBuilder::new();

    attrs
        .add(
            Attribute::BatadvAttrMeshIfindex,
            AttrValueForSend::U32(ifindex),
        )
        .map_err(|_| Error::Netlink("Failed to add MeshIfIndex attribute".to_string()))?;

    let msg = netlink::build_genl_msg(Command::BatadvCmdGetOriginators, attrs.build())
        .map_err(|_| Error::Netlink("Failed to build netlink message".to_string()))?;

    let mut socket = netlink::BatadvSocket::connect()
        .await
        .map_err(|_| Error::Netlink("Failed to connect to batman-adv socket".to_string()))?;

    let mut response = socket
        .send(NlmF::REQUEST | NlmF::DUMP, msg)
        .await
        .map_err(|_| Error::Netlink("Failed to send netlink request".to_string()))?;

    let mut originators: Vec<Originator> = Vec::new();
    while let Some(msg) = response.next().await {
        let msg: Nlmsghdr<u16, Genlmsghdr<u8, u16>> =
            msg.map_err(|_| Error::Netlink("Failed to parse netlink message".to_string()))?;

        match *msg.nl_type() {
            x if x == u16::from(Nlmsg::Done) => break,
            x if x == u16::from(Nlmsg::Error) => match &msg.nl_payload() {
                NlPayload::Err(err) if *err.error() == 0 => break,
                NlPayload::Err(err) => {
                    return Err(Error::Netlink(format!("Netlink error {}", err.error())));
                }
                _ => {
                    return Err(Error::Netlink("Unknown netlink error payload".to_string()));
                }
            },
            _ => {}
        }

        let attrs = msg
            .get_payload()
            .ok_or_else(|| Error::Argument("Message without payload".into()))?
            .attrs()
            .get_attr_handle();

        let orig = attrs
            .get_attr_payload_as::<[u8; 6]>(Attribute::BatadvAttrOrigAddress.into())
            .map_err(|_| Error::Argument("Missing ORIG_ADDRESS".into()))?;

        let neigh = attrs
            .get_attr_payload_as::<[u8; 6]>(Attribute::BatadvAttrNeighAddress.into())
            .map_err(|_| Error::Argument("Missing NEIGH_ADDRESS".into()))?;

        let outgoing_if =
            match attrs.get_attr_payload_as::<[u8; 16]>(Attribute::BatadvAttrHardIfname.into()) {
                Ok(bytes) => {
                    let nul_pos = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
                    String::from_utf8_lossy(&bytes[..nul_pos]).into_owned()
                }
                Err(_) => {
                    let idx = attrs
                        .get_attr_payload_as::<u32>(Attribute::BatadvAttrHardIfindex.into())
                        .map_err(|_| Error::Argument("Missing HARD_IFINDEX".into()))?;
                    if_indextoname(idx).await.map_err(|_| {
                        Error::Netlink(format!("Failed to resolve ifindex {} -> name", idx))
                    })?
                }
            };

        let last_seen_ms = attrs
            .get_attr_payload_as::<u32>(Attribute::BatadvAttrLastSeenMsecs.into())
            .map_err(|_| Error::Argument("Missing LAST_SEEN_MSECS".into()))?;

        let tq = attrs
            .get_attr_payload_as::<u8>(Attribute::BatadvAttrTq.into())
            .ok();
        let tp = attrs
            .get_attr_payload_as::<u32>(Attribute::BatadvAttrThroughput.into())
            .ok();
        let is_best = attrs
            .get_attribute(Attribute::BatadvAttrFlagBest.into())
            .is_some();

        originators.push(Originator {
            originator: MacAddr6::from(orig),
            next_hop: MacAddr6::from(neigh),
            outgoing_if,
            last_seen_ms,
            tq,
            throughput: tp,
            is_best,
        });
    }

    Ok(originators)
}
