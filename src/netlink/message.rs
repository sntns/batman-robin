use crate::error::Error;
use crate::model::Command;

use neli::consts::nl::NlmF;
use neli::genl::{Genlmsghdr, GenlmsghdrBuilder};
use neli::nl::{NlPayload, Nlmsghdr, NlmsghdrBuilder};
use neli::types::{Buffer, GenlBuffer};

/// Builds a Generic Netlink message for a given BATMAN-adv command.
///
/// # Parameters
/// - `cmd`: The `Command` to send (e.g., `BatadvCmdGetOriginators`).
/// - `attrs`: A `GenlBuffer` containing all attributes to include in the message.
///
/// # Returns
/// - `Ok(Genlmsghdr<u8, u16>)` containing the constructed Generic Netlink header with payload.
/// - `Err(Error)` if building the GENL header fails.
///
/// # Notes
/// The `attrs` should include at least the mesh interface index or name (`BatadvAttrMeshIfindex`
/// or `BatadvAttrMeshIfname`) as required by BATMAN-adv. Additional attributes can be added
/// according to the BATMAN-adv API.
pub fn build_genl_msg(
    cmd: Command,
    attrs: GenlBuffer<u16, Buffer>,
) -> Result<Genlmsghdr<u8, u16>, Error> {
    // Build GENL header with attributes
    let genl_msg = GenlmsghdrBuilder::default()
        .cmd(cmd.into())
        .version(1)
        .attrs(attrs)
        .build()
        .map_err(|e| Error::Netlink(format!("Failed to build GENL header: {:?}", e)))?;

    Ok(genl_msg)
}

/// Builds a full Netlink message wrapping a Generic Netlink message.
///
/// # Parameters
/// - `family_id`: The numeric ID of the Generic Netlink family (e.g., BATMAN-adv).
/// - `cmd`: The `Command` to send.
/// - `attrs`: Attributes to include in the message.
/// - `seq`: Sequence number for tracking the Netlink message.
///
/// # Returns
/// - `Ok(Nlmsghdr<u16, Genlmsghdr<u8, u16>>)` ready to be sent via a Netlink socket.
/// - `Err(Error)` if building either the GENL or NL headers fails.
///
/// # Notes
/// - Sets `NLM_F_REQUEST | NLM_F_DUMP` flags: `REQUEST` signals a request, `DUMP` is for
///   multi-entry responses.
/// - `nl_pid` is set to 0 so the kernel fills in the sender PID automatically.
#[allow(dead_code)]
pub fn build_nl_msg(
    family_id: u16,
    cmd: Command,
    attrs: GenlBuffer<u16, Buffer>,
    seq: u32,
) -> Result<Nlmsghdr<u16, Genlmsghdr<u8, u16>>, Error> {
    // Build GENL header with attributes
    let genl_msg = build_genl_msg(cmd, attrs)
        .map_err(|e| Error::Netlink(format!("Failed to build GENL header: {:?}", e)))?;

    // Build Netlink header with payload
    let nl_msg = NlmsghdrBuilder::default()
        .nl_type(family_id)
        .nl_flags(NlmF::REQUEST | NlmF::DUMP) // REQUEST for query, DUMP if expecting multiple entries
        .nl_seq(seq) // sequence number, can be incremented externally
        .nl_pid(0) // 0 lets the kernel fill in the sender PID
        .nl_payload(NlPayload::Payload(genl_msg))
        .build()
        .map_err(|e| Error::Netlink(format!("Failed to build NL header: {:?}", e)))?;

    Ok(nl_msg)
}
