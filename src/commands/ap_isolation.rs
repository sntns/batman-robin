use crate::error::Error;
use crate::model::{AttrValueForSend, Attribute, Command};
use crate::netlink;

use neli::consts::nl::NlmF;
use neli::genl::Genlmsghdr;
use neli::nl::Nlmsghdr;

/// Retrieves the current state of AP (Access Point) isolation for a BATMAN-adv mesh interface.
///
/// # Arguments
///
/// * `ifindex` - The mesh interface index.
///
/// # Returns
///
/// Returns `Ok(true)` if AP isolation is enabled, `Ok(false)` if disabled,
/// or a `Error` if the value could not be retrieved.
pub async fn get_ap_isolation(ifindex: u32) -> Result<bool, Error> {
    let mut attrs = netlink::GenlAttrBuilder::new();
    attrs
        .add(
            Attribute::BatadvAttrMeshIfindex,
            AttrValueForSend::U32(ifindex),
        )
        .map_err(|_| Error::Netlink("Error - could not set mesh interface index".to_string()))?;

    let msg = netlink::build_genl_msg(Command::BatadvCmdGetMeshInfo, attrs.build())
        .map_err(|_| Error::Netlink("Error - failed to build netlink message".to_string()))?;

    let mut sock = netlink::BatadvSocket::connect().await.map_err(|_| {
        Error::Netlink("Error - failed to connect to batman-adv netlink socket".to_string())
    })?;

    let mut response = sock
        .send(NlmF::REQUEST, msg)
        .await
        .map_err(|_| Error::Netlink("Error - failed to send netlink request".to_string()))?;

    while let Some(msg) = response.next().await {
        let msg: Nlmsghdr<u16, Genlmsghdr<u8, u16>> = msg
            .map_err(|_| Error::Netlink("Error - failed to parse netlink response".to_string()))?;

        let payload = match msg.get_payload() {
            Some(p) => p,
            None => continue,
        };

        for attr in payload.attrs().iter() {
            if *attr.nla_type().nla_type() == u16::from(Attribute::BatadvAttrApIsolationEnabled) {
                let bytes = attr.nla_payload().as_ref();
                if let Some(&val) = bytes.first() {
                    return Ok(val != 0);
                }
            }
        }
    }

    Err(Error::NotFound(
        "Error - AP isolation attribute not found".to_string(),
    ))
}

/// Enables or disables AP (Access Point) isolation for a BATMAN-adv mesh interface.
///
/// # Arguments
///
/// * `ifindex` - The mesh interface index.
/// * `enabled` - `true` to enable AP isolation, `false` to disable.
///
/// # Returns
///
/// Returns `Ok(())` if the operation succeeds, or a `Error` if it fails.
pub async fn set_ap_isolation(ifindex: u32, enabled: bool) -> Result<(), Error> {
    let mut attrs = netlink::GenlAttrBuilder::new();
    attrs
        .add(
            Attribute::BatadvAttrMeshIfindex,
            AttrValueForSend::U32(ifindex),
        )
        .map_err(|_| Error::Netlink("Error - could not set mesh interface index".to_string()))?;

    attrs
        .add(
            Attribute::BatadvAttrApIsolationEnabled,
            AttrValueForSend::U8(enabled.into()),
        )
        .map_err(|_| Error::Netlink("Error - could not set AP isolation attribute".to_string()))?;

    let msg = netlink::build_genl_msg(Command::BatadvCmdSetMesh, attrs.build())
        .map_err(|_| Error::Netlink("Error - failed to build netlink message".to_string()))?;

    let mut sock = netlink::BatadvSocket::connect().await.map_err(|_| {
        Error::Netlink("Error - failed to connect to batman-adv netlink socket".to_string())
    })?;

    sock.send(NlmF::REQUEST | NlmF::ACK, msg)
        .await
        .map_err(|_| Error::Netlink("Error - failed to send netlink request".to_string()))?;

    Ok(())
}
