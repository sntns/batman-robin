//! Udev adapter for gateway events (primary implementation).
//!
//! This adapter listens to kernel uevents via the generic netlink socket
//! and filters for batman-adv gateway events.
//!
//! # Kernel Event Format
//!
//! Batman-adv emits gateway uevents with:
//! - ACTION: "change"
//! - SUBSYSTEM: "net"
//! - BATTYPE: "gw"
//! - BATACTION: "ADD" | "CHANGE" | "DEL"
//! - BATDATA: MAC address (for ADD/CHANGE)

use crate::commands::utils::if_indextoname;
use crate::error::Error;
use crate::gateway_events::GatewayEventService;
use crate::model::{GatewayEvent, GatewayEventAction};
use async_trait::async_trait;
use futures::stream::BoxStream;
use macaddr::MacAddr;
use neli::consts::socket::{Msg, NlFamily};
use neli::socket::NlSocket;
use neli::utils::Groups;
use std::collections::HashMap;
use std::str::FromStr;
use tokio::io::unix::AsyncFd;
use tokio::sync::mpsc::unbounded_channel;
use tokio_stream::wrappers::UnboundedReceiverStream;

/// Gateway event listener using kernel uevents via `NETLINK_KOBJECT_UEVENT`.
///
/// This is the primary implementation. It listens to batman-adv gateway
/// change events directly from the kernel, providing real-time notifications
/// with minimal overhead.
pub struct UeventListener;

const UEVENT_MCAST_GROUP: u32 = 1;
const UEVENT_BUFFER_SIZE: usize = 16 * 1024;

impl UeventListener {
    /// Subscribe to gateway events via kernel uevents.
    #[tracing::instrument()]
    pub async fn subscribe_events(
        meshif_index: u32,
    ) -> Result<BoxStream<'static, Result<GatewayEvent, Error>>, Error> {
        let meshif_name = if_indextoname(meshif_index).await?;

        let socket = NlSocket::connect(
            NlFamily::KobjectUevent,
            None,
            Groups::new_groups(&[UEVENT_MCAST_GROUP]),
        )
        .map_err(|e| Error::Netlink(format!("failed to connect to uevent socket: {e}")))?;

        socket
            .nonblock()
            .map_err(|e| Error::Io(format!("failed to mark uevent socket non-blocking: {e}")))?;

        socket
            .set_recv_buffer_size(UEVENT_BUFFER_SIZE * 4)
            .map_err(|e| {
                Error::Io(format!(
                    "failed to configure uevent socket receive buffer: {e}"
                ))
            })?;

        let socket = AsyncFd::new(socket)
            .map_err(|e| Error::Io(format!("failed to register uevent socket: {e}")))?;

        let (tx, rx) = unbounded_channel();

        tokio::spawn(async move {
            tracing::debug!(
                "Udev adapter: starting gateway event listener for {}",
                meshif_name
            );

            let mut buffer = vec![0u8; UEVENT_BUFFER_SIZE];

            loop {
                let bytes = loop {
                    let mut guard = match socket.readable().await {
                        Ok(guard) => guard,
                        Err(e) => {
                            let _ = tx.send(Err(Error::Io(format!(
                                "failed waiting for uevent socket readability: {e}"
                            ))));
                            return;
                        }
                    };

                    match guard
                        .try_io(|inner| inner.get_ref().recv(buffer.as_mut_slice(), Msg::empty()))
                    {
                        Ok(Ok((bytes, _groups))) => break bytes,
                        Ok(Err(e)) => {
                            let _ = tx.send(Err(Error::Io(format!(
                                "failed receiving uevent payload: {e}"
                            ))));
                            return;
                        }
                        Err(_would_block) => continue,
                    }
                };

                if bytes == 0 {
                    tracing::debug!("Uevent socket returned EOF for {}", meshif_name);
                    return;
                }

                let properties = match Self::parse_uevent_properties(&buffer[..bytes]) {
                    Ok(properties) => properties,
                    Err(e) => {
                        tracing::trace!("Ignoring malformed uevent payload: {e}");
                        continue;
                    }
                };

                if properties.get("SUBSYSTEM").map(String::as_str) != Some("net") {
                    continue;
                }

                if properties.get("BATTYPE").map(String::as_str) != Some("gw") {
                    continue;
                }

                let matches_interface = properties
                    .get("INTERFACE")
                    .map(String::as_str)
                    .is_some_and(|interface| interface == meshif_name)
                    || properties
                        .get("IFINDEX")
                        .and_then(|ifindex| ifindex.parse::<u32>().ok())
                        == Some(meshif_index);

                if !matches_interface {
                    continue;
                }

                let event = match Self::parse_gateway_event(meshif_index, properties) {
                    Ok(event) => event,
                    Err(e) => {
                        tracing::warn!(
                            "Ignoring invalid gateway uevent for {}: {}",
                            meshif_name,
                            e
                        );
                        continue;
                    }
                };

                if tx.send(Ok(event)).is_err() {
                    tracing::debug!(
                        "Gateway event receiver dropped for {}, stopping listener",
                        meshif_name
                    );
                    return;
                }
            }
        });

        Ok(Box::pin(UnboundedReceiverStream::new(rx)))
    }

    /// Parse null-separated key=value pairs from kernel uevent.
    ///
    /// Kernel uevents are encoded as a sequence of null-terminated strings,
    /// each of the form "key=value".
    fn parse_uevent_properties(data: &[u8]) -> Result<HashMap<String, String>, Error> {
        let text = String::from_utf8_lossy(data);
        let properties: HashMap<String, String> = text
            .split('\0')
            .filter_map(|line| {
                let (k, v) = line.split_once('=')?;
                Some((k.to_string(), v.to_string()))
            })
            .collect();

        if properties.is_empty() {
            return Err(Error::Argument("empty uevent properties".to_string()));
        }

        Ok(properties)
    }

    /// Convert parsed uevent properties to GatewayEvent.
    fn parse_gateway_event(
        meshif_index: u32,
        props: HashMap<String, String>,
    ) -> Result<GatewayEvent, Error> {
        // Verify this is a gateway event
        if props.get("BATTYPE").map(|s| s.as_str()) != Some("gw") {
            return Err(Error::Argument("not a gateway uevent".to_string()));
        }

        // Extract action
        let action_str = props
            .get("BATACTION")
            .ok_or(Error::Argument("missing BATACTION".to_string()))?;

        let action = match action_str.to_uppercase().as_str() {
            "ADD" => GatewayEventAction::Add,
            "CHANGE" => GatewayEventAction::Change,
            "DEL" => GatewayEventAction::Delete,
            _ => {
                return Err(Error::Argument(format!(
                    "unknown BATACTION: {}",
                    action_str
                )));
            }
        };

        // Extract gateway MAC (present for ADD/CHANGE, absent for DEL)
        let gateway_mac = props.get("BATDATA").and_then(|s| MacAddr::from_str(s).ok());

        Ok(GatewayEvent {
            timestamp: std::time::SystemTime::now(),
            meshif: meshif_index,
            action,
            gateway_mac,
        })
    }
}

#[async_trait]
impl GatewayEventService for UeventListener {
    async fn subscribe_gateway_events(
        &self,
        meshif: u32,
    ) -> Result<BoxStream<'static, Result<GatewayEvent, Error>>, Error> {
        Self::subscribe_events(meshif).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_uevent_properties() {
        let data = b"ACTION=change\0BATTYPE=gw\0BATACTION=ADD\0BATDATA=60:09:c3:aa:bb:cc\0";
        let props = UeventListener::parse_uevent_properties(data).unwrap();

        assert_eq!(props.get("ACTION"), Some(&"change".to_string()));
        assert_eq!(props.get("BATTYPE"), Some(&"gw".to_string()));
        assert_eq!(props.get("BATACTION"), Some(&"ADD".to_string()));
        assert_eq!(props.get("BATDATA"), Some(&"60:09:c3:aa:bb:cc".to_string()));
    }

    #[test]
    fn test_parse_gateway_event_add() {
        let mut props = HashMap::new();
        props.insert("BATTYPE".to_string(), "gw".to_string());
        props.insert("BATACTION".to_string(), "ADD".to_string());
        props.insert("BATDATA".to_string(), "60:09:c3:aa:bb:cc".to_string());

        let event = UeventListener::parse_gateway_event(6, props).unwrap();

        assert_eq!(event.meshif, 6);
        assert_eq!(event.action, GatewayEventAction::Add);
        assert!(event.gateway_mac.is_some());
    }

    #[test]
    fn test_parse_gateway_event_delete() {
        let mut props = HashMap::new();
        props.insert("BATTYPE".to_string(), "gw".to_string());
        props.insert("BATACTION".to_string(), "DEL".to_string());

        let event = UeventListener::parse_gateway_event(6, props).unwrap();

        assert_eq!(event.meshif, 6);
        assert_eq!(event.action, GatewayEventAction::Delete);
        assert!(event.gateway_mac.is_none());
    }

    #[test]
    fn test_invalid_action() {
        let mut props = HashMap::new();
        props.insert("BATTYPE".to_string(), "gw".to_string());
        props.insert("BATACTION".to_string(), "INVALID".to_string());

        let result = UeventListener::parse_gateway_event(6, props);
        assert!(result.is_err());
    }
}
