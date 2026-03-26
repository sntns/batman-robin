use crate::error::Error;
use crate::model::{AttrValueForSend, Attribute, ClientFlags, Command, TransglobalEntry};
use crate::netlink;

use macaddr::MacAddr6;
use neli::consts::nl::NlmF;
use neli::consts::nl::Nlmsg;
use neli::genl::Genlmsghdr;
use neli::nl::NlPayload;
use neli::nl::Nlmsghdr;

/// Retrieves the global translation table (TT) entries for a given BATMAN-adv mesh interface.
///
/// This corresponds to the `batctl tg` command and returns information about all known clients
/// and their originators in the mesh network.
///
/// # Arguments
///
/// * `ifindex` - The mesh interface index.
///
/// # Returns
///
/// A vector of `TransglobalEntry` structs, each containing:
/// - `client`: The MAC address of the client.
/// - `orig`: The originator MAC address for the client.
/// - `vid`: The VLAN ID associated with the client.
/// - `ttvn`: The current translation table version for this client.
/// - `last_ttvn`: The last observed translation table version.
/// - `flags`: Client flags (e.g., roaming, isolated).
/// - `crc32`: CRC32 checksum of the entry.
/// - `is_best`: Indicates if this entry is marked as the "best" path.
///
/// Returns a `Error` if any netlink operation or parsing fails.
pub async fn get_transglobal(ifindex: u32) -> Result<Vec<TransglobalEntry>, Error> {
    let mut attrs = netlink::GenlAttrBuilder::new();

    attrs
        .add(
            Attribute::BatadvAttrMeshIfindex,
            AttrValueForSend::U32(ifindex),
        )
        .map_err(|_| Error::Netlink("Failed to add MeshIfIndex attribute".to_string()))?;

    let msg = netlink::build_genl_msg(Command::BatadvCmdGetTranstableGlobal, attrs.build())
        .map_err(|_| Error::Netlink("Failed to build Netlink message".to_string()))?;

    let mut sock = netlink::BatadvSocket::connect().await.map_err(|_| {
        Error::Netlink("Failed to connect to batman-adv Netlink socket".to_string())
    })?;

    let mut response = sock
        .send(NlmF::REQUEST | NlmF::DUMP, msg)
        .await
        .map_err(|_| Error::Netlink("Failed to send Netlink request".to_string()))?;

    let mut entries = Vec::new();
    while let Some(msg) = response.next().await {
        let msg: Nlmsghdr<u16, Genlmsghdr<u8, u16>> =
            msg.map_err(|_| Error::Netlink("Failed to parse Netlink message".to_string()))?;

        match *msg.nl_type() {
            x if x == u16::from(Nlmsg::Done) => break,
            x if x == u16::from(Nlmsg::Error) => match &msg.nl_payload() {
                NlPayload::Err(err) if *err.error() == 0 => break,
                NlPayload::Err(err) => {
                    return Err(Error::Netlink(format!("Netlink error {}", err.error())));
                }
                _ => {
                    return Err(Error::Netlink("Unknown Netlink error payload".to_string()));
                }
            },
            _ => {}
        }

        let attrs = msg
            .get_payload()
            .ok_or_else(|| Error::Argument("Message without payload".to_string()))?
            .attrs()
            .get_attr_handle();

        let client = attrs
            .get_attr_payload_as::<[u8; 6]>(Attribute::BatadvAttrTtAddress.into())
            .map_err(|_| Error::Argument("Missing TT_ADDRESS".to_string()))?;
        let orig = attrs
            .get_attr_payload_as::<[u8; 6]>(Attribute::BatadvAttrOrigAddress.into())
            .map_err(|_| Error::Argument("Missing ORIG_ADDRESS".to_string()))?;
        let vid = attrs
            .get_attr_payload_as::<u16>(Attribute::BatadvAttrTtVid.into())
            .map_err(|_| Error::Argument("Missing TT_VID".to_string()))?;
        let ttvn = attrs
            .get_attr_payload_as::<u8>(Attribute::BatadvAttrTtTtvn.into())
            .map_err(|_| Error::Argument("Missing TT_TTVN".to_string()))?;
        let last_ttvn = attrs
            .get_attr_payload_as::<u8>(Attribute::BatadvAttrTtLastTtvn.into())
            .map_err(|_| Error::Argument("Missing TT_LAST_TTVN".to_string()))?;
        let crc32 = attrs
            .get_attr_payload_as::<u32>(Attribute::BatadvAttrTtCrc32.into())
            .map_err(|_| Error::Argument("Missing TT_CRC32".to_string()))?;
        let raw_flags = attrs
            .get_attr_payload_as::<u32>(Attribute::BatadvAttrTtFlags.into())
            .map_err(|_| Error::Argument("Missing TT_FLAGS".to_string()))?;
        let flags = ClientFlags::from_bits_truncate(raw_flags);
        let is_best = attrs
            .get_attribute(Attribute::BatadvAttrFlagBest.into())
            .is_some();

        entries.push(TransglobalEntry {
            client: MacAddr6::from(client),
            orig: MacAddr6::from(orig),
            vid,
            ttvn,
            last_ttvn,
            flags,
            crc32,
            is_best,
        });
    }

    Ok(entries)
}
