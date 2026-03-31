use crate::error::Error;

use neli::consts::{
    nl::NlmF,
    rtnl::{Ifla, IflaInfo, RtAddrFamily, Rtm},
    socket::NlFamily,
};
use neli::nl::{NlPayload, Nlmsghdr};
use neli::router::asynchronous::NlRouter;
use neli::rtnl::{Ifinfomsg, IfinfomsgBuilder, RtattrBuilder};
use neli::types::{Buffer, RtBuffer};
use neli::utils::Groups;

/// Creates a new BATMAN-adv mesh interface.
///
/// Optionally, a routing algorithm can be specified. This corresponds to `ip link add type batadv`.
///
/// # Arguments
///
/// * `mesh_if` - The name of the mesh interface to create.
/// * `routing_algo` - Optional routing algorithm name (e.g., `"BATMAN_IV"`).
///
/// # Returns
///
/// Returns `Ok(())` on success, or a `Error` if creation fails.
///
/// # Example
///
/// ```no_run
/// # async fn example() {
/// // create_mesh("bat0", Some("BATMAN_IV")).await?;
/// # }
/// ```
pub async fn create_mesh(mesh_if: &str, routing_algo: Option<&str>) -> Result<(), Error> {
    const IFLA_BATADV_ALGO_NAME: u16 = 1;
    let (rtnl, _) = NlRouter::connect(NlFamily::Route, None, Groups::empty())
        .await
        .map_err(|_| Error::Netlink("Error - failed to connect to netlink router".to_string()))?;

    rtnl.enable_ext_ack(true)
        .map_err(|_| Error::Netlink("Error - failed to enable extended ACK".to_string()))?;
    rtnl.enable_strict_checking(true)
        .map_err(|_| Error::Netlink("Error - failed to enable strict checking".to_string()))?;

    let ifname_attr = RtattrBuilder::default()
        .rta_type(Ifla::Ifname)
        .rta_payload(mesh_if)
        .build()
        .map_err(|_| Error::Netlink("Error - failed to build IFNAME attribute".to_string()))?;

    let kind_attr = RtattrBuilder::default()
        .rta_type(IflaInfo::Kind)
        .rta_payload("batadv")
        .build()
        .map_err(|_| Error::Netlink("Error - failed to build INFO_KIND attribute".to_string()))?;

    let mut info_data_attrs: RtBuffer<u16, Buffer> = RtBuffer::new();
    if let Some(algo) = routing_algo {
        let algo_attr = RtattrBuilder::default()
            .rta_type(IFLA_BATADV_ALGO_NAME)
            .rta_payload(algo)
            .build()
            .map_err(|_| {
                Error::Netlink("Error - failed to build ALGO_NAME attribute".to_string())
            })?;
        info_data_attrs.push(algo_attr);
    }

    let info_data_attr = RtattrBuilder::default()
        .rta_type(IflaInfo::Data)
        .rta_payload(info_data_attrs)
        .build()
        .map_err(|_| Error::Netlink("Error - failed to build INFO_DATA attribute".to_string()))?;

    let mut linkinfo_attrs: RtBuffer<IflaInfo, Buffer> = RtBuffer::new();
    linkinfo_attrs.push(kind_attr);
    linkinfo_attrs.push(info_data_attr);

    let linkinfo_attr = RtattrBuilder::default()
        .rta_type(Ifla::Linkinfo)
        .rta_payload(linkinfo_attrs)
        .build()
        .map_err(|_| Error::Netlink("Error - failed to build LINKINFO attribute".to_string()))?;

    let mut rtattrs: RtBuffer<Ifla, Buffer> = RtBuffer::new();
    rtattrs.push(ifname_attr);
    rtattrs.push(linkinfo_attr);

    let msg = IfinfomsgBuilder::default()
        .ifi_family(RtAddrFamily::Unspecified)
        .rtattrs(rtattrs)
        .build()
        .map_err(|_| Error::Netlink("Error - failed to build Ifinfomsg".to_string()))?;

    rtnl.send::<_, _, Rtm, Ifinfomsg>(
        Rtm::Newlink,
        NlmF::REQUEST | NlmF::CREATE | NlmF::EXCL | NlmF::ACK,
        NlPayload::Payload(msg),
    )
    .await
    .map_err(|_| Error::Netlink("Error - failed to create mesh interface".to_string()))?;

    Ok(())
}

/// Deletes an existing BATMAN-adv mesh interface.
///
/// This corresponds to `ip link delete <mesh_if>`.
///
/// # Arguments
///
/// * `mesh_ifindex` - The interface index of the mesh interface to delete.
///
/// # Returns
///
/// Returns `Ok(())` on success, or a `Error` if deletion fails.
///
/// # Example
///
/// ```no_run
/// # async fn example() {
/// // delete_mesh("bat0").await?;
/// # }
/// ```
pub async fn delete_mesh(mesh_ifindex: u32) -> Result<(), Error> {
    let (rtnl, _) = NlRouter::connect(NlFamily::Route, None, Groups::empty())
        .await
        .map_err(|_| Error::Netlink("Error - failed to connect to netlink router".to_string()))?;

    rtnl.enable_ext_ack(true)
        .map_err(|_| Error::Netlink("Error - failed to enable extended ACK".to_string()))?;
    rtnl.enable_strict_checking(true)
        .map_err(|_| Error::Netlink("Error - failed to enable strict checking".to_string()))?;

    let msg = IfinfomsgBuilder::default()
        .ifi_family(RtAddrFamily::Unspecified)
        .ifi_index(mesh_ifindex.cast_signed())
        .build()
        .map_err(|_| Error::Netlink("Error - failed to build Ifinfomsg".to_string()))?;

    rtnl.send::<_, _, Rtm, Ifinfomsg>(
        Rtm::Dellink,
        NlmF::REQUEST | NlmF::ACK,
        NlPayload::Payload(msg),
    )
    .await
    .map_err(|_| Error::Netlink("Error - failed to destroy mesh interface".to_string()))?;

    Ok(())
}

/// Lists BATMAN-adv mesh interfaces present on the host.
///
/// This inspects Linux links via `RTM_GETLINK` and keeps only interfaces whose
/// `IFLA_INFO_KIND` is `"batadv"`. The returned entries use the `Interface` model,
/// where:
/// - `ifname` is the mesh interface name (for example, `bat0`)
/// - `active` indicates whether the interface exists in the kernel link table
///
/// # Returns
///
/// Returns a vector of mesh interfaces detected on the system.
///
/// # Errors
///
/// Returns [`Error::Netlink`] if netlink connection, request, or response parsing fails.
///
/// # Example
///
/// ```no_run
/// # use batman_robin::Client;
/// # async fn example() -> Result<(), batman_robin::Error> {
/// let client = Client::new();
/// let meshes = client.mesh_list().await?;
/// for mesh in meshes {
///     println!("mesh={}", mesh);
/// }
/// # Ok(())
/// # }
/// ```
pub async fn list_meshes() -> Result<Vec<String>, Error> {
    let (rtnl, _) = NlRouter::connect(NlFamily::Route, None, Groups::empty())
        .await
        .map_err(|_| Error::Netlink("Error - failed to connect to netlink router".to_string()))?;

    rtnl.enable_ext_ack(true)
        .map_err(|_| Error::Netlink("Error - failed to enable extended ACK".to_string()))?;
    rtnl.enable_strict_checking(true)
        .map_err(|_| Error::Netlink("Error - failed to enable strict checking".to_string()))?;

    let msg = IfinfomsgBuilder::default()
        .ifi_family(RtAddrFamily::Unspecified)
        .build()
        .map_err(|_| Error::Netlink("Error - failed to build Ifinfomsg".to_string()))?;

    let mut response = rtnl
        .send::<_, _, Rtm, Ifinfomsg>(
            Rtm::Getlink,
            NlmF::REQUEST | NlmF::DUMP | NlmF::ACK,
            NlPayload::Payload(msg),
        )
        .await
        .map_err(|_| Error::Netlink("Error - failed to send Getlink request".to_string()))?;

    let mut meshes = Vec::new();

    while let Some(msg) = response.next().await {
        let msg: Nlmsghdr<Rtm, Ifinfomsg> =
            msg.map_err(|_| Error::Netlink("Error - failed to parse netlink message".to_string()))?;

        let payload = match msg.get_payload() {
            Some(payload) => payload,
            None => continue,
        };

        let attrs = payload.rtattrs().get_attr_handle();

        let ifname = match attrs.get_attr_payload_as_with_len::<String>(Ifla::Ifname) {
            Ok(ifname) => ifname,
            Err(_) => continue,
        };

        let linkinfo = match attrs.get_nested_attributes::<IflaInfo>(Ifla::Linkinfo) {
            Ok(linkinfo) => linkinfo,
            Err(_) => continue,
        };

        let kind = match linkinfo.get_attr_payload_as_with_len::<String>(IflaInfo::Kind) {
            Ok(kind) => kind,
            Err(_) => continue,
        };

        if kind != "batadv" {
            continue;
        }

        meshes.push(ifname);
    }

    Ok(meshes)
}
