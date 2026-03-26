use crate::error::Error;
use crate::{Attribute, Command, netlink};

use crate::commands::get_algoname_netlink;
use crate::netlink::GenlAttrBuilder;
use neli::consts::{
    nl::NlmF,
    rtnl::{Ifla, IflaInfo, RtAddrFamily, Rtm},
    socket::NlFamily,
};
use neli::genl::Genlmsghdr;
use neli::nl::{NlPayload, Nlmsghdr};
use neli::router::asynchronous::NlRouter;
use neli::rtnl::{Ifinfomsg, IfinfomsgBuilder};
use neli::utils::Groups;
use std::fs;

/// Returns the default routing algorithm configured for BATMAN-adv.
///
/// Reads the value from `/sys/module/batman_adv/parameters/routing_algo`.
///
/// # Returns
///
/// A `String` representing the default routing algorithm, or a `Error` if reading fails.
pub async fn get_default_routing_algo() -> Result<String, Error> {
    let path = "/sys/module/batman_adv/parameters/routing_algo";

    let content = fs::read_to_string(path).map_err(|e| {
        Error::Io(format!(
            "Failed to read default routing algo from {}: {}",
            path, e
        ))
    })?;

    Ok(content.trim().to_string())
}

/// Returns a list of currently active BATMAN-adv mesh interfaces and their routing algorithms.
///
/// Each entry is a tuple `(mesh_interface_name, algorithm_name)`.
///
/// # Returns
///
/// A vector of tuples, or a `Error` if querying fails.
///
/// # Notes
///
/// Only interfaces of kind `"batadv"` are included.
pub async fn get_active_routing_algos() -> Result<Vec<(String, String)>, Error> {
    let (rtnl, _) = NlRouter::connect(NlFamily::Route, None, Groups::empty())
        .await
        .map_err(|e| Error::Netlink(format!("Failed to connect to Netlink: {:?}", e)))?;

    rtnl.enable_ext_ack(true)
        .map_err(|e| Error::Netlink(format!("Failed to enable extended ACK: {:?}", e)))?;
    rtnl.enable_strict_checking(true)
        .map_err(|e| Error::Netlink(format!("Failed to enable strict checking: {:?}", e)))?;

    let msg = IfinfomsgBuilder::default()
        .ifi_family(RtAddrFamily::Unspecified)
        .build()
        .map_err(|e| Error::Netlink(format!("Failed to build Ifinfomsg: {:?}", e)))?;

    let mut response = rtnl
        .send::<_, _, Rtm, Ifinfomsg>(
            Rtm::Getlink,
            NlmF::REQUEST | NlmF::DUMP | NlmF::ACK,
            NlPayload::Payload(msg),
        )
        .await
        .map_err(|e| Error::Netlink(format!("Failed to send GETLINK request: {:?}", e)))?;

    let mut result = Vec::new();
    while let Some(msg) = response.next().await {
        let msg: Nlmsghdr<Rtm, Ifinfomsg> =
            msg.map_err(|e| Error::Netlink(format!("Failed to parse Netlink message: {:?}", e)))?;

        let payload = match msg.get_payload() {
            Some(p) => p,
            None => continue,
        };

        let attrs = payload.rtattrs().get_attr_handle();
        let mesh_if = match attrs.get_attr_payload_as_with_len::<String>(Ifla::Ifname) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let linkinfo = match attrs.get_nested_attributes::<IflaInfo>(Ifla::Linkinfo) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let kind = match linkinfo.get_attr_payload_as_with_len::<String>(IflaInfo::Kind) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if kind != "batadv" {
            continue;
        }

        let algo = get_algoname_netlink(mesh_if.as_str()).await.map_err(|e| {
            Error::Netlink(format!(
                "Failed to get routing algorithm for '{}': {:?}",
                mesh_if, e
            ))
        })?;

        result.push((mesh_if, algo));
    }

    Ok(result)
}

/// Returns all routing algorithms available in the kernel module.
///
/// Queries BATMAN-adv via netlink to list all supported routing algorithms.
///
/// # Returns
///
/// A vector of algorithm names as `String`s, or a `Error` if none are found or the query fails.
pub async fn get_available_routing_algos() -> Result<Vec<String>, Error> {
    let msg = netlink::build_genl_msg(
        Command::BatadvCmdGetRoutingAlgos,
        GenlAttrBuilder::new().build(),
    )
    .map_err(|e| Error::Netlink(format!("Failed to build routing algos request: {:?}", e)))?;

    let mut sock = netlink::BatadvSocket::connect().await.map_err(|e| {
        Error::Netlink(format!(
            "Failed to connect to batadv netlink socket: {:?}",
            e
        ))
    })?;

    let mut response = sock
        .send(NlmF::REQUEST | NlmF::DUMP, msg)
        .await
        .map_err(|e| Error::Netlink(format!("Failed to send routing algos request: {:?}", e)))?;

    let mut algos = Vec::new();
    while let Some(msg) = response.next().await {
        let msg: Nlmsghdr<u16, Genlmsghdr<u8, u16>> = msg.map_err(|e| {
            Error::Netlink(format!("Failed to parse routing algos message: {:?}", e))
        })?;

        let payload = match msg.get_payload() {
            Some(p) => p,
            None => continue,
        };

        for attr in payload.attrs().iter() {
            if *attr.nla_type().nla_type() == u16::from(Attribute::BatadvAttrAlgoName) {
                let bytes = attr.nla_payload().as_ref();
                let nul = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
                let algo = String::from_utf8_lossy(&bytes[..nul]).to_string();
                algos.push(algo);
            }
        }
    }

    if algos.is_empty() {
        return Err(Error::NotFound("No routing algorithms found".to_string()));
    }

    Ok(algos)
}

/// Sets the default routing algorithm for BATMAN-adv.
///
/// Writes the specified algorithm name to
/// `/sys/module/batman_adv/parameters/routing_algo`.
///
/// # Arguments
///
/// * `algo` - The name of the routing algorithm to set as default.
///
/// # Returns
///
/// Returns `()` on success, or a `Error` if writing fails.
pub async fn set_default_routing_algo(algo: &str) -> Result<(), Error> {
    let path = "/sys/module/batman_adv/parameters/routing_algo";

    fs::write(path, algo).map_err(|e| {
        Error::Io(format!(
            "Failed to set default routing algo to '{}': {}",
            algo, e
        ))
    })?;

    Ok(())
}
