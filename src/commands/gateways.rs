use crate::commands::if_indextoname;
use crate::error::Error;
use crate::model::{AttrValueForSend, Attribute, Command, Gateway};
use crate::netlink;

use macaddr::MacAddr6;
use neli::consts::nl::{NlmF, Nlmsg};
use neli::genl::Genlmsghdr;
use neli::nl::{NlPayload, Nlmsghdr};

/// Retrieves the list of gateways known to a BATMAN-adv mesh interface.
///
/// This corresponds to the `batctl gwl` command. Each entry contains information
/// about the gateway's MAC address, associated router, outgoing interface, bandwidth,
/// throughput, TQ (routing metric), and whether it is currently marked as the best gateway.
///
/// # Arguments
///
/// * `ifindex` - The mesh interface index.
///
/// # Returns
///
/// Returns a vector of `Gateway` structs representing all gateways the mesh node is aware of,
/// or a `Error` if the information could not be retrieved or parsed.
///
/// # Example
///
/// ```no_run
/// # use batman_robin::model::Gateway;
/// # async fn example() {
/// # let gateways: Vec<Gateway> = vec![];
/// // let gateways = get_gateways_list("bat0").await?;
/// for gw in gateways {
///     println!("Gateway {} via {} (best: {})", gw.mac_addr, gw.outgoing_if, gw.is_best);
/// }
/// # }
/// ```
pub async fn get_gateways_list(ifindex: u32) -> Result<Vec<Gateway>, Error> {
    let mut attrs = netlink::GenlAttrBuilder::new();

    attrs
        .add(
            Attribute::BatadvAttrMeshIfindex,
            AttrValueForSend::U32(ifindex),
        )
        .map_err(|_| Error::Netlink("Error - could not set mesh interface index".to_string()))?;

    let msg = netlink::build_genl_msg(Command::BatadvCmdGetGateways, attrs.build())
        .map_err(|_| Error::Netlink("Error - failed to build netlink message".to_string()))?;

    let mut socket = netlink::BatadvSocket::connect().await.map_err(|_| {
        Error::Netlink("Error - failed to connect to batman-adv netlink socket".to_string())
    })?;

    let mut response = socket
        .send(NlmF::REQUEST | NlmF::DUMP, msg)
        .await
        .map_err(|_| Error::Netlink("Error - failed to send netlink request".to_string()))?;

    let mut gateways = Vec::new();

    while let Some(msg) = response.next().await {
        let msg: Nlmsghdr<u16, Genlmsghdr<u8, u16>> = msg
            .map_err(|_| Error::Netlink("Error - failed to parse netlink response".to_string()))?;

        match *msg.nl_type() {
            x if x == u16::from(Nlmsg::Done) => break,
            x if x == u16::from(Nlmsg::Error) => {
                match &msg.nl_payload() {
                    NlPayload::Err(err) if *err.error() == 0 => break, // end of dump
                    NlPayload::Err(err) => {
                        return Err(Error::Netlink(format!("Netlink error {}", err.error())));
                    }
                    _ => {
                        return Err(Error::Netlink("Unknown netlink error payload".to_string()));
                    }
                }
            }
            _ => {}
        }

        let attrs = msg
            .get_payload()
            .ok_or_else(|| Error::Argument("Error - netlink message has no payload".into()))?
            .attrs()
            .get_attr_handle();

        let is_best = attrs
            .get_attribute(Attribute::BatadvAttrFlagBest.into())
            .is_some();

        let mac_addr = attrs
            .get_attr_payload_as::<[u8; 6]>(Attribute::BatadvAttrOrigAddress.into())
            .map_err(|_| Error::Argument("Error - gateway originator address missing".into()))?;

        let router = attrs
            .get_attr_payload_as::<[u8; 6]>(Attribute::BatadvAttrRouter.into())
            .map_err(|_| Error::Argument("Error - gateway router address missing".into()))?;

        let outgoing_if =
            match attrs.get_attr_payload_as::<[u8; 16]>(Attribute::BatadvAttrHardIfname.into()) {
                Ok(bytes) => {
                    let nul_pos = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
                    String::from_utf8_lossy(&bytes[..nul_pos]).into_owned()
                }
                Err(_) => {
                    let ifindex = attrs
                        .get_attr_payload_as::<u32>(Attribute::BatadvAttrHardIfindex.into())
                        .map_err(|_| {
                            Error::Argument("Error - gateway hard interface index missing".into())
                        })?;
                    if_indextoname(ifindex).await.map_err(|_| {
                        Error::Netlink(
                            "Error - failed to resolve interface name from index".to_string(),
                        )
                    })?
                }
            };

        let bandwidth_down = attrs
            .get_attr_payload_as::<u32>(Attribute::BatadvAttrBandwidthDown.into())
            .ok();
        let bandwidth_up = attrs
            .get_attr_payload_as::<u32>(Attribute::BatadvAttrBandwidthUp.into())
            .ok();
        let throughput = attrs
            .get_attr_payload_as::<u32>(Attribute::BatadvAttrThroughput.into())
            .ok();
        let tq = attrs
            .get_attr_payload_as::<u8>(Attribute::BatadvAttrTq.into())
            .ok();

        gateways.push(Gateway {
            mac_addr: MacAddr6::from(mac_addr),
            router: MacAddr6::from(router),
            outgoing_if,
            bandwidth_down,
            bandwidth_up,
            throughput,
            tq,
            is_best,
        });
    }

    Ok(gateways)
}
