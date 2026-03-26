use crate::error::Error;
use crate::model::{AttrValueForSend, Attribute, Command};
use crate::netlink;
use neli::consts::nl::NlmF;
use neli::consts::rtnl::{Ifla, RtAddrFamily, Rtm};
use neli::consts::socket::NlFamily;
use neli::genl::Genlmsghdr;
use neli::nl::{NlPayload, Nlmsghdr};
use neli::router::asynchronous::NlRouter;
use neli::rtnl::{Ifinfomsg, IfinfomsgBuilder};
use neli::utils::Groups;

/// Retrieves the routing algorithm name associated with a given BATMAN-adv mesh interface.
///
/// This function queries the netlink interface for the specified mesh interface and
/// returns the algorithm name currently in use.
///
/// # Arguments
///
/// * `mesh_if` - The name of the BATMAN-adv mesh interface.
///
/// # Returns
///
/// A `String` containing the algorithm name, or a `Error` if the interface
/// cannot be queried or the algorithm name cannot be found.
pub async fn get_algoname_netlink(mesh_if: &str) -> Result<String, Error> {
    let ifindex = super::if_nametoindex(mesh_if).await.map_err(|_| {
        Error::Netlink(format!(
            "Error - interface '{}' is not present or not a batman-adv interface",
            mesh_if
        ))
    })?;

    let mut attrs = netlink::GenlAttrBuilder::new();
    attrs
        .add(
            Attribute::BatadvAttrMeshIfindex,
            AttrValueForSend::U32(ifindex),
        )
        .map_err(|_| Error::Netlink("Failed to add MeshIfIndex attribute".to_string()))?;

    let msg = netlink::build_genl_msg(Command::BatadvCmdGetMeshInfo, attrs.build())
        .map_err(|_| Error::Netlink("Failed to build Netlink message".to_string()))?;

    let mut sock = netlink::BatadvSocket::connect().await.map_err(|_| {
        Error::Netlink("Failed to connect to batman-adv Netlink socket".to_string())
    })?;

    let mut response = sock
        .send(NlmF::REQUEST, msg)
        .await
        .map_err(|_| Error::Netlink("Failed to send Netlink request".to_string()))?;

    while let Some(msg) = response.next().await {
        let msg: Nlmsghdr<u16, Genlmsghdr<u8, u16>> =
            msg.map_err(|_| Error::Netlink("Failed to parse Netlink message".to_string()))?;

        let payload = match msg.get_payload() {
            Some(p) => p,
            None => continue,
        };

        for attr in payload.attrs().iter() {
            if *attr.nla_type().nla_type() == u16::from(Attribute::BatadvAttrAlgoName) {
                let bytes = attr.nla_payload().as_ref();
                let nul = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
                return Ok(String::from_utf8_lossy(&bytes[..nul]).to_string());
            }
        }
    }

    Err(Error::NotFound(format!(
        "No routing algorithm found for interface '{}'",
        mesh_if
    )))
}

/// Converts a network interface name to its corresponding interface index (ifindex).
///
/// This function uses netlink to enumerate all interfaces and find the index
/// matching the provided interface name.
///
/// # Arguments
///
/// * `ifname` - The name of the network interface.
///
/// # Returns
///
/// The `u32` interface index corresponding to `ifname`, or a `Error` if
/// the interface does not exist or a netlink operation fails.
pub async fn if_nametoindex(ifname: &str) -> Result<u32, Error> {
    let (rtnl, _) = NlRouter::connect(NlFamily::Route, None, Groups::empty())
        .await
        .map_err(|_| Error::Netlink("Failed to connect to Netlink".to_string()))?;

    rtnl.enable_ext_ack(true).ok();
    rtnl.enable_strict_checking(true).ok();

    let ifinfomsg = IfinfomsgBuilder::default()
        .ifi_family(RtAddrFamily::Unspecified)
        .build()
        .map_err(|_| Error::Netlink("Failed to create Ifinfomsg".to_string()))?;

    let mut response = rtnl
        .send::<_, _, Rtm, Ifinfomsg>(
            Rtm::Getlink,
            NlmF::DUMP | NlmF::ACK,
            NlPayload::Payload(ifinfomsg),
        )
        .await
        .map_err(|_| Error::Netlink("Failed to send Netlink request".to_string()))?;

    while let Some(msg) = response.next().await {
        let msg: Nlmsghdr<Rtm, Ifinfomsg> =
            msg.map_err(|_| Error::Netlink("Failed to parse Netlink message".to_string()))?;

        if let Some(payload) = msg.get_payload() {
            let attrs = payload.rtattrs().get_attr_handle();
            if let Ok(name) = attrs.get_attr_payload_as_with_len::<String>(Ifla::Ifname)
                && name == ifname
            {
                return Ok(payload.ifi_index().cast_unsigned());
            }
        }
    }

    Err(Error::NotFound(format!("Interface '{}' not found", ifname)))
}

/// Converts a network interface index (ifindex) to its corresponding interface name.
///
/// This function uses netlink to enumerate all interfaces and find the name
/// matching the provided interface index.
///
/// # Arguments
///
/// * `ifindex` - The index of the network interface.
///
/// # Returns
///
/// A `String` with the interface name corresponding to `ifindex`, or a `Error` if
/// the interface does not exist or a netlink operation fails.
pub async fn if_indextoname(ifindex: u32) -> Result<String, Error> {
    let (rtnl, _) = NlRouter::connect(NlFamily::Route, None, Groups::empty())
        .await
        .map_err(|_| Error::Netlink("Failed to connect to Netlink".to_string()))?;

    rtnl.enable_ext_ack(true).ok();
    rtnl.enable_strict_checking(true).ok();

    let ifinfomsg = IfinfomsgBuilder::default()
        .ifi_family(RtAddrFamily::Unspecified)
        .build()
        .map_err(|_| Error::Netlink("Failed to create Ifinfomsg".to_string()))?;

    let mut response = rtnl
        .send::<_, _, Rtm, Ifinfomsg>(
            Rtm::Getlink,
            NlmF::DUMP | NlmF::ACK,
            NlPayload::Payload(ifinfomsg),
        )
        .await
        .map_err(|_| Error::Netlink("Failed to send Netlink request".to_string()))?;

    while let Some(msg) = response.next().await {
        let msg: Nlmsghdr<Rtm, Ifinfomsg> =
            msg.map_err(|_| Error::Netlink("Failed to parse Netlink message".to_string()))?;

        if let Some(payload) = msg.get_payload()
            && *payload.ifi_index() == ifindex.cast_signed()
        {
            let attrs = payload.rtattrs().get_attr_handle();
            if let Ok(name) = attrs.get_attr_payload_as_with_len::<String>(Ifla::Ifname) {
                return Ok(name);
            }
        }
    }

    Err(Error::NotFound(format!(
        "Interface with index {} not found",
        ifindex
    )))
}
