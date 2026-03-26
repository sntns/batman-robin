use macaddr::MacAddr;
use std::time::SystemTime;

/// Gateway change event emitted by batman-adv kernel module.
///
/// When a batman-adv node is in gateway client mode, it emits uevents
/// when the selected gateway changes. Applications can subscribe to these
/// events to perform actions like starting/renewing DHCP leases.
///
/// # Examples
///
/// ```ignore
/// let client = batman_robin::Client::new();
/// let mut events = client
///     .subscribe_gateway_events(batman_robin::MeshSelector::with_name("bat0"))
///     .await?;
///
/// while let Some(event) = events.next().await {
///     match event?.action {
///         GatewayEventAction::Add => println!("Gateway selected: {:?}", event?.gateway_mac),
///         GatewayEventAction::Change => println!("Better gateway found"),
///         GatewayEventAction::Delete => println!("Gateway lost"),
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayEvent {
    /// Timestamp when the event was received
    pub timestamp: SystemTime,

    /// Mesh interface index (e.g. from `if_nametoindex("bat0")`)
    pub meshif: u32,

    /// Type of gateway change event
    pub action: GatewayEventAction,

    /// MAC address of the selected gateway (present for ADD/CHANGE, absent for DELETE)
    pub gateway_mac: Option<MacAddr>,
}

/// Gateway event action types
///
/// These correspond to batman-adv's BATACTION uevent field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum GatewayEventAction {
    /// First gateway was selected (ADD)
    ///
    /// This event is sent when a batman-adv node transitions to having a selected
    /// gateway. Applications should typically start a DHCP client on this event.
    Add = 1,

    /// A better gateway was found (CHANGE)
    ///
    /// This event is sent when a batman-adv node switches to a different gateway
    /// due to improved link quality or other selection criteria.
    /// Applications should typically renew their DHCP lease on this event.
    Change = 2,

    /// The selected gateway is no longer available (DEL)
    ///
    /// This event is sent when the currently selected gateway disappears
    /// and no alternative gateway is available. The `gateway_mac` field
    /// will be `None` for this action.
    Delete = 3,
}

impl GatewayEvent {
    /// Create a new gateway event
    pub fn new(meshif: u32, action: GatewayEventAction, gateway_mac: Option<MacAddr>) -> Self {
        Self {
            timestamp: SystemTime::now(),
            meshif,
            action,
            gateway_mac,
        }
    }

    /// Check if this event has a valid associated gateway MAC
    pub fn has_gateway(&self) -> bool {
        self.gateway_mac.is_some()
    }
}

impl std::fmt::Display for GatewayEventAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Add => write!(f, "ADD"),
            Self::Change => write!(f, "CHANGE"),
            Self::Delete => write!(f, "DELETE"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gateway_event_creation() {
        let mac = "60:09:c3:aa:bb:cc".parse().unwrap();
        let event = GatewayEvent::new(6, GatewayEventAction::Add, Some(mac));

        assert_eq!(event.meshif, 6);
        assert_eq!(event.action, GatewayEventAction::Add);
        assert_eq!(event.gateway_mac, Some(mac));
        assert!(event.has_gateway());
    }

    #[test]
    fn test_delete_event_no_gateway() {
        let event = GatewayEvent::new(6, GatewayEventAction::Delete, None);

        assert_eq!(event.action, GatewayEventAction::Delete);
        assert!(!event.has_gateway());
    }

    #[test]
    fn test_action_display() {
        assert_eq!(GatewayEventAction::Add.to_string(), "ADD");
        assert_eq!(GatewayEventAction::Change.to_string(), "CHANGE");
        assert_eq!(GatewayEventAction::Delete.to_string(), "DELETE");
    }
}
