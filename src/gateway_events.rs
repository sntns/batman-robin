//! Gateway event subscription service.
//!
//! This module provides async subscriptions to batman-adv gateway change events.
//! Batman-adv emits uevents when the selected gateway changes in client mode,
//! allowing applications to react to topology changes.
//!
//! # Overview
//!
//! Batman-adv natively emits gateway change events via the kernel's netlink
//! kobject uevent mechanism (NETLINK_KOBJECT_UEVENT). This module abstracts
//! that mechanism through a trait-based service interface.
//!
//! # Backend
//!
//! Gateway events are provided via batman-adv's native
//! `NETLINK_KOBJECT_UEVENT` notifications.
//!
//! # Example
//!
//! ```ignore
//! use batman_robin::Client;
//! use futures::stream::StreamExt;
//!
//! async fn listen_gateways() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = Client::new();
//!
//!     // Subscribe to gateway events
//!     let mut events = client
//!         .subscribe_gateway_events(batman_robin::MeshSelector::with_name("bat0"))
//!         .await?;
//!
//!     while let Some(result) = events.next().await {
//!         match result {
//!             Ok(event) => println!("Gateway event: {:?}", event),
//!             Err(e) => eprintln!("Event error: {}", e),
//!         }
//!     }
//!     Ok(())
//! }
//! ```

use crate::error::Error;
use crate::model::GatewayEvent;
use async_trait::async_trait;
use futures::stream::BoxStream;

/// Service for subscribing to gateway change events.
///
/// Implementations listen to batman-adv kernel uevents and emit
/// GatewayEvent when the selected gateway changes.
#[async_trait]
pub trait GatewayEventService: Send + Sync {
    /// Subscribe to gateway change events for a mesh interface.
    ///
    /// Returns an async stream that emits `GatewayEvent` whenever batman-adv
    /// detects a gateway change (selection, replacement, or loss).
    ///
    /// The returned stream never terminates as long as the interface exists
    /// and batman-adv is active. It should be gracefully cancelled when
    /// no longer needed.
    ///
    /// # Arguments
    ///
    /// * `meshif` - Mesh interface index (e.g., from `if_nametoindex("bat0")`)
    ///
    /// # Returns
    ///
    /// - `Ok(stream)`: Async stream of gateway events
    /// - `Err(Error)`: If subscription setup fails (for example due to
    ///   insufficient privileges or an invalid interface)
    ///
    /// # Errors
    ///
    /// - `Error::Netlink`: Underlying netlink/udev system error
    /// - `Error::NotFound`: Mesh interface not found
    /// - `Error::Io`: Underlying socket setup or read failure
    async fn subscribe_gateway_events(
        &self,
        meshif: u32,
    ) -> Result<BoxStream<'static, Result<GatewayEvent, Error>>, Error>;
}
