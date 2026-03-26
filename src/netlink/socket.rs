use crate::error::Error;

use neli::consts::nl::NlmF;
use neli::consts::socket::NlFamily;
use neli::genl::Genlmsghdr;
use neli::nl::NlPayload;
use neli::router::asynchronous::{NlRouter, NlRouterReceiverHandle};
use neli::utils::Groups;

/// Async wrapper around a Generic Netlink socket for interacting with BATMAN-adv.
///
/// Provides methods to connect to the `batadv` family and send messages,
/// returning an async handle to receive responses.
pub struct BatadvSocket {
    sock: NlRouter,
    family_id: u16,
}

impl BatadvSocket {
    /// Connects to the Generic Netlink `batadv` family.
    ///
    /// Resolves the family ID for `batadv` and prepares the socket for sending messages.
    ///
    /// # Returns
    /// - `Ok(Self)` on success with an initialized `BatadvSocket`.
    /// - `Err(Error)` if the connection or family resolution fails.
    pub async fn connect() -> Result<Self, Error> {
        let (sock, _mcast) = NlRouter::connect(NlFamily::Generic, None, Groups::empty())
            .await
            .map_err(|e| Error::Netlink(format!("Failed to connect with NlRouter: {:?}", e)))?;
        let family_id = sock
            .resolve_genl_family("batadv")
            .await
            .map_err(|e| Error::Netlink(format!("Failed to resolve family: {:?}", e)))?;

        Ok(Self { sock, family_id })
    }

    /// Sends a Generic Netlink message to the `batadv` family.
    ///
    /// # Parameters
    /// - `flags`: Flags controlling message behavior (`NlmF::REQUEST`, `NlmF::DUMP`, etc.).
    /// - `msg`: The Generic Netlink message to send (`Genlmsghdr<u8, u16>`).
    ///
    /// # Returns
    /// - `Ok(NlRouterReceiverHandle)` to asynchronously iterate over responses.
    /// - `Err(Error)` if sending the message fails.
    pub async fn send(
        &mut self,
        flags: NlmF,
        msg: Genlmsghdr<u8, u16>,
    ) -> Result<NlRouterReceiverHandle<u16, Genlmsghdr<u8, u16>>, Error> {
        let recv = self
            .sock
            .send(self.family_id, flags, NlPayload::Payload(msg))
            .await
            .map_err(|e| Error::Netlink(format!("Failed to send message: {:?}", e)))?;

        Ok(recv)
    }
}
