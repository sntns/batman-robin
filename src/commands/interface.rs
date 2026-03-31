use crate::commands::if_indextoname;
use crate::error::Error;
use crate::model::{AttrValueForSend, Attribute, Command, Interface};
use crate::netlink;

use neli::consts::{
    nl::{NlmF, Nlmsg},
    rtnl::{Ifla, RtAddrFamily, Rtm},
    socket::NlFamily,
};
use neli::genl::Genlmsghdr;
use neli::nl::{NlPayload, Nlmsghdr};
use neli::router::asynchronous::NlRouter;
use neli::rtnl::{Ifinfomsg, IfinfomsgBuilder, RtattrBuilder};
use neli::types::{Buffer, RtBuffer};
use neli::utils::Groups;

/// Counts the number of physical or virtual interfaces attached to a BATMAN-adv mesh interface.
///
/// # Arguments
///
/// * `mesh_ifindex` - The mesh interface index.
///
/// # Returns
///
/// Returns the number of interfaces currently enslaved to the given mesh interface,
/// or a `Error` if the query fails.
///
/// # Example
///
/// ```no_run
/// # async fn example() {
/// # let count = 0u32;
/// // let count = count_interfaces("bat0").await?;
/// println!("Number of interfaces: {}", count);
/// # }
/// ```
pub async fn count_interfaces(mesh_ifindex: u32) -> Result<u32, Error> {
    let (rtnl, _) = NlRouter::connect(NlFamily::Route, None, Groups::empty())
        .await
        .map_err(|_| Error::Netlink("Error - failed to connect to netlink router".to_string()))?;

    rtnl.enable_ext_ack(true)
        .map_err(|_| Error::Netlink("Error - failed to enable extended ACK".to_string()))?;
    rtnl.enable_strict_checking(true)
        .map_err(|_| Error::Netlink("Error - failed to enable strict checking".to_string()))?;

    let ifinfomsg = IfinfomsgBuilder::default()
        .ifi_family(RtAddrFamily::Unspecified)
        .build()
        .map_err(|_| Error::Netlink("Error - failed to build Ifinfomsg".to_string()))?;

    let mut response = rtnl
        .send::<_, _, Rtm, Ifinfomsg>(
            Rtm::Getlink,
            NlmF::DUMP | NlmF::ACK,
            NlPayload::Payload(ifinfomsg),
        )
        .await
        .map_err(|_| Error::Netlink("Error - failed to send Getlink request".to_string()))?;

    let mut count = 0u32;
    while let Some(msg) = response.next().await {
        let msg: Nlmsghdr<Rtm, Ifinfomsg> =
            msg.map_err(|_| Error::Netlink("Error - failed to parse netlink message".to_string()))?;

        if let Some(payload) = msg.get_payload() {
            let attrs = payload.rtattrs().get_attr_handle();
            if let Ok(master) = attrs.get_attr_payload_as::<u32>(Ifla::Master)
                && master == mesh_ifindex
            {
                count += 1;
            }
        }
    }

    Ok(count)
}

/// Retrieves the list of interfaces associated with a BATMAN-adv mesh interface.
///
/// This corresponds to the `batctl if` command. Each entry contains the interface name
/// and whether it is currently active.
///
/// # Arguments
///
/// * `mesh_ifindex` - The mesh interface index.
///
/// # Returns
///
/// Returns a vector of `Interface` structs or a `Error` if the query fails.
///
/// # Example
///
/// ```no_run
/// # use batman_robin::model::Interface;
/// # async fn example() {
/// # let ifaces: Vec<Interface> = vec![];
/// // let ifaces = get_interfaces("bat0").await?;
/// for iface in ifaces {
///     println!("Interface {} active: {}", iface.ifname, iface.active);
/// }
/// # }
/// ```
pub async fn get_interfaces(mesh_ifindex: u32) -> Result<Vec<Interface>, Error> {
    let mut attrs = netlink::GenlAttrBuilder::new();

    attrs
        .add(
            Attribute::BatadvAttrMeshIfindex,
            AttrValueForSend::U32(mesh_ifindex),
        )
        .map_err(|_| Error::Netlink("Error - failed to add MeshIfindex attribute".to_string()))?;

    let msg = netlink::build_genl_msg(Command::BatadvCmdGetHardif, attrs.build())
        .map_err(|_| Error::Netlink("Error - failed to build netlink message".to_string()))?;

    let mut sock = netlink::BatadvSocket::connect().await.map_err(|_| {
        Error::Netlink("Error - failed to connect to batman-adv socket".to_string())
    })?;

    let mut response = sock
        .send(NlmF::REQUEST | NlmF::DUMP, msg)
        .await
        .map_err(|_| Error::Netlink("Error - failed to send netlink request".to_string()))?;

    let mut interfaces = Vec::new();
    while let Some(msg) = response.next().await {
        let msg: Nlmsghdr<u16, Genlmsghdr<u8, u16>> =
            msg.map_err(|_| Error::Netlink("Error - failed to parse netlink message".to_string()))?;

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
            .ok_or_else(|| Error::Argument("Error - message has no payload".into()))?
            .attrs()
            .get_attr_handle();

        let hard_ifindex = attrs
            .get_attr_payload_as::<u32>(Attribute::BatadvAttrHardIfindex.into())
            .map_err(|_| Error::Argument("Error - missing HARD_IFINDEX".into()))?;

        let ifname = if_indextoname(hard_ifindex).await.map_err(|_| {
            Error::Netlink(format!(
                "Error - failed to resolve interface index {}",
                hard_ifindex
            ))
        })?;

        let active = attrs
            .get_attribute(Attribute::BatadvAttrActive.into())
            .is_some();

        interfaces.push(Interface { ifname, active });
    }

    Ok(interfaces)
}

/// Adds or removes a physical interface from a BATMAN-adv mesh interface.
///
/// This corresponds to `batctl if add` or `batctl if del`.
///
/// # Arguments
///
/// * `iface_ifindex` - The index of the interface to add or remove.
/// * `mesh_ifindex` - Optional mesh interface index to attach to. `None` removes it from any mesh.
///
/// # Returns
///
/// Returns `Ok(())` on success, or a `Error` if the operation fails.
///
/// # Example
///
/// ```no_run
/// # async fn example() {
/// // set_interface(3, Some(5)).await?;
/// // set_interface(3, None).await?; // remove from mesh
/// # }
/// ```
pub async fn set_interface(iface_ifindex: u32, mesh_ifindex: Option<u32>) -> Result<(), Error> {
    let mesh_ifindex = mesh_ifindex.unwrap_or(0);

    let (rtnl, _) = NlRouter::connect(NlFamily::Route, None, Groups::empty())
        .await
        .map_err(|_| Error::Netlink("Error - failed to connect to netlink router".to_string()))?;

    rtnl.enable_ext_ack(true)
        .map_err(|_| Error::Netlink("Error - failed to enable extended ACK".to_string()))?;
    rtnl.enable_strict_checking(true)
        .map_err(|_| Error::Netlink("Error - failed to enable strict checking".to_string()))?;

    let master_attr = RtattrBuilder::default()
        .rta_type(Ifla::Master)
        .rta_payload(mesh_ifindex)
        .build()
        .map_err(|_| Error::Netlink("Error - failed to build Master attribute".to_string()))?;

    let mut rtattrs: RtBuffer<Ifla, Buffer> = RtBuffer::new();
    rtattrs.push(master_attr);

    let msg = IfinfomsgBuilder::default()
        .ifi_family(RtAddrFamily::Unspecified)
        .ifi_index(iface_ifindex.cast_signed())
        .rtattrs(rtattrs)
        .build()
        .map_err(|_| Error::Netlink("Error - failed to build Ifinfomsg".to_string()))?;

    rtnl.send::<_, _, Rtm, Ifinfomsg>(
        Rtm::Setlink,
        NlmF::REQUEST | NlmF::ACK,
        NlPayload::Payload(msg),
    )
    .await
    .map_err(|_| Error::Netlink("Error - failed to set interface".to_string()))?;

    Ok(())
}
