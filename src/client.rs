use super::Error;
use crate::commands;
use crate::model;
use futures::stream::BoxStream;
use validator::Validate;

/// High-level client for interacting with the BATMAN-adv mesh network.
///
/// `Client` provides asynchronous methods to query and configure BATMAN-adv
/// interfaces and settings via netlink.
///
/// Mesh-targeted methods take a [`model::MeshSelector`] by value. You can build
/// selectors explicitly with [`model::MeshSelector::with_name`] or
/// [`model::MeshSelector::with_ifindex`].
///
/// # Example
///
/// ```no_run
/// use batman_robin::{Client, MeshSelector};
///
/// # async fn example() -> Result<(), batman_robin::Error> {
/// let client = Client::new();
/// let selector = MeshSelector::with_name("bat0");
///
/// let neighbors = client.neighbors(selector.clone()).await?;
/// println!("{} neighbor entries", neighbors.len());
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Client;

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    /// Creates a new instance of `Client`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use batman_robin::Client;
    ///
    /// let client = Client::new();
    /// let _ = client;
    /// ```
    pub fn new() -> Self {
        Self {}
    }

    /// Resolves a `MeshSelector` into a concrete interface index.
    ///
    /// This method validates the selector first, then resolves by `ifindex` directly
    /// when present or by interface `name` via `if_nametoindex`.
    ///
    /// # Arguments
    /// * `selector` - Mesh selector (by name and/or ifindex).
    ///
    /// # Errors
    /// Returns [`Error::Argument`] if validation fails.
    /// Returns [`Error::Netlink`] if name resolution fails.
    async fn selector_to_ifindex(&self, selector: model::MeshSelector) -> Result<u32, Error> {
        selector
            .validate()
            .map_err(|err| Error::Argument(err.to_string()))?;

        if let Some(ifindex) = selector.ifindex {
            return Ok(ifindex);
        }

        if let Some(name) = selector.name {
            return commands::if_nametoindex(name.as_str()).await.map_err(|_| {
                Error::Netlink(format!(
                    "Error - interface '{}' is not present or not a batman-adv interface",
                    name
                ))
            });
        }

        Err(Error::Argument("Invalid selector".to_string()))
    }

    /// Resolves an `InterfaceSelector` into a concrete interface index.
    ///
    /// This method validates the selector first, then resolves by `ifindex` directly
    /// when present or by interface `name` via `if_nametoindex`.
    ///
    /// # Arguments
    /// * `selector` - Interface selector (by name and/or ifindex).
    ///
    /// # Errors
    /// Returns [`Error::Argument`] if validation fails.
    /// Returns [`Error::Netlink`] if name resolution fails.
    async fn interface_selector_to_ifindex(
        &self,
        selector: model::InterfaceSelector,
    ) -> Result<u32, Error> {
        selector
            .validate()
            .map_err(|err| Error::Argument(err.to_string()))?;

        if let Some(ifindex) = selector.ifindex {
            return Ok(ifindex);
        }

        if let Some(name) = selector.name {
            return commands::if_nametoindex(name.as_str()).await.map_err(|_| {
                Error::Netlink(format!("Error - interface '{}' is not present", name))
            });
        }

        Err(Error::Argument("Invalid selector".to_string()))
    }

    /// Retrieves the list of originators for the selected mesh interface.
    ///
    /// # Arguments
    /// * `selector` - Mesh selector.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use batman_robin::{Client, MeshSelector};
    ///
    /// # async fn example() -> Result<(), batman_robin::Error> {
    /// let client = Client::new();
    /// let entries = client.originators(MeshSelector::with_name("bat0")).await?;
    /// println!("{} originators", entries.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn originators(
        &self,
        selector: model::MeshSelector,
    ) -> Result<Vec<model::Originator>, Error> {
        let ifindex = self.selector_to_ifindex(selector).await?;
        commands::get_originators(ifindex).await
    }

    /// Retrieves the list of gateways for the selected mesh interface.
    ///
    /// # Arguments
    /// * `selector` - Mesh selector.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use batman_robin::{Client, MeshSelector};
    ///
    /// # async fn example() -> Result<(), batman_robin::Error> {
    /// let client = Client::new();
    /// let gateways = client.gateways(MeshSelector::with_name("bat0")).await?;
    /// println!("{} gateways", gateways.len());
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(skip(self))]
    pub async fn gateways(
        &self,
        selector: model::MeshSelector,
    ) -> Result<Vec<model::Gateway>, Error> {
        let ifindex = self.selector_to_ifindex(selector).await?;
        commands::get_gateways_list(ifindex).await
    }

    /// Subscribes to gateway change events for the selected mesh interface.
    ///
    /// # Arguments
    /// * `selector` - Mesh selector.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use batman_robin::{Client, MeshSelector};
    /// use futures::StreamExt;
    ///
    /// # async fn example() -> Result<(), batman_robin::Error> {
    /// let client = Client::new();
    /// let mut events = client
    ///     .subscribe_gateway_events(MeshSelector::with_name("bat0"))
    ///     .await?;
    ///
    /// while let Some(event) = events.next().await {
    ///     println!("{:?}", event?);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(skip(self))]
    pub async fn subscribe_gateway_events(
        &self,
        selector: model::MeshSelector,
    ) -> Result<BoxStream<'static, Result<model::GatewayEvent, Error>>, Error> {
        let ifindex = self.selector_to_ifindex(selector).await?;
        commands::UeventListener::subscribe_events(ifindex).await
    }

    /// Gets current gateway mode and related configuration for the selected mesh interface.
    ///
    /// # Arguments
    /// * `selector` - Mesh selector.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use batman_robin::{Client, MeshSelector};
    ///
    /// # async fn example() -> Result<(), batman_robin::Error> {
    /// let client = Client::new();
    /// let gw = client.get_gw_mode(MeshSelector::with_name("bat0")).await?;
    /// println!("mode={:?}", gw.mode);
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(skip(self))]
    pub async fn get_gw_mode(
        &self,
        selector: model::MeshSelector,
    ) -> Result<model::GatewayInfo, Error> {
        let ifindex = self.selector_to_ifindex(selector).await?;
        commands::get_gateway(ifindex).await
    }

    /// Sets gateway mode and optional parameters for the selected mesh interface.
    ///
    /// # Arguments
    /// * `selector` - Mesh selector.
    /// * `mode` - Gateway mode to apply.
    /// * `down` - Optional downstream bandwidth parameter.
    /// * `up` - Optional upstream bandwidth parameter.
    /// * `sel_class` - Optional gateway selection class.
    ///
    /// # Errors
    /// Returns [`Error`] if selector validation, selector resolution, or netlink write fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use batman_robin::{Client, GwMode, MeshSelector};
    ///
    /// # async fn example() -> Result<(), batman_robin::Error> {
    /// let client = Client::new();
    /// client
    ///     .set_gw_mode(
    ///         MeshSelector::with_name("bat0"),
    ///         GwMode::Client,
    ///         None,
    ///         None,
    ///         Some(20),
    ///     )
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_gw_mode(
        &self,
        selector: model::MeshSelector,
        mode: model::GwMode,
        down: Option<u32>,
        up: Option<u32>,
        sel_class: Option<u32>,
    ) -> Result<(), Error> {
        let ifindex = self.selector_to_ifindex(selector).await?;
        commands::set_gateway(mode, down, up, sel_class, ifindex).await
    }

    /// Retrieves global translation table entries for the selected mesh interface.
    ///
    /// # Arguments
    /// * `selector` - Mesh selector.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use batman_robin::{Client, MeshSelector};
    ///
    /// # async fn example() -> Result<(), batman_robin::Error> {
    /// let client = Client::new();
    /// let tg = client.transglobal(MeshSelector::with_name("bat0")).await?;
    /// println!("{} global entries", tg.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn transglobal(
        &self,
        selector: model::MeshSelector,
    ) -> Result<Vec<model::TransglobalEntry>, Error> {
        let ifindex = self.selector_to_ifindex(selector).await?;
        commands::get_transglobal(ifindex).await
    }

    /// Retrieves local translation table entries for the selected mesh interface.
    ///
    /// # Arguments
    /// * `selector` - Mesh selector.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use batman_robin::{Client, MeshSelector};
    ///
    /// # async fn example() -> Result<(), batman_robin::Error> {
    /// let client = Client::new();
    /// let tl = client.translocal(MeshSelector::with_name("bat0")).await?;
    /// println!("{} local entries", tl.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn translocal(
        &self,
        selector: model::MeshSelector,
    ) -> Result<Vec<model::TranslocalEntry>, Error> {
        let ifindex = self.selector_to_ifindex(selector).await?;
        commands::get_translocal(ifindex).await
    }

    /// Retrieves the list of neighbors for the selected mesh interface.
    ///
    /// # Arguments
    /// * `selector` - Mesh selector.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use batman_robin::{Client, MeshSelector};
    ///
    /// # async fn example() -> Result<(), batman_robin::Error> {
    /// let client = Client::new();
    /// let neighbors = client.neighbors(MeshSelector::with_name("bat0")).await?;
    /// println!("{} neighbors", neighbors.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn neighbors(
        &self,
        selector: model::MeshSelector,
    ) -> Result<Vec<model::Neighbor>, Error> {
        let ifindex = self.selector_to_ifindex(selector).await?;
        commands::get_neighbors(ifindex).await
    }

    /// Retrieves the list of physical interfaces attached to the selected mesh interface.
    ///
    /// # Arguments
    /// * `selector` - Mesh selector.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use batman_robin::{Client, MeshSelector};
    ///
    /// # async fn example() -> Result<(), batman_robin::Error> {
    /// let client = Client::new();
    /// let ifaces = client.interface_list(MeshSelector::with_name("bat0")).await?;
    /// println!("{} attached interfaces", ifaces.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn interface_list(
        &self,
        selector: model::MeshSelector,
    ) -> Result<Vec<model::Interface>, Error> {
        let ifindex = self.selector_to_ifindex(selector).await?;
        commands::get_interfaces(ifindex).await
    }

    /// Adds a physical interface to a selected mesh interface.
    ///
    /// # Arguments
    /// * `selector` - Mesh selector identifying the target mesh interface.
    /// * `interface_selector` - Interface selector for the physical interface to add.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use batman_robin::{Client, InterfaceSelector, MeshSelector};
    ///
    /// # async fn example() -> Result<(), batman_robin::Error> {
    /// let client = Client::new();
    /// client
    ///     .interface_add(
    ///         MeshSelector::with_name("bat0"),
    ///         InterfaceSelector::with_name("wlan0"),
    ///     )
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn interface_add(
        &self,
        selector: model::MeshSelector,
        interface_selector: model::InterfaceSelector,
    ) -> Result<(), Error> {
        let mesh_ifindex = self.selector_to_ifindex(selector).await?;
        let iface_ifindex = self
            .interface_selector_to_ifindex(interface_selector)
            .await?;

        commands::set_interface(iface_ifindex, Some(mesh_ifindex)).await
    }

    /// Removes a physical interface from any mesh interface.
    ///
    /// # Arguments
    /// * `interface_selector` - Interface selector for the physical interface to remove.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use batman_robin::{Client, InterfaceSelector};
    ///
    /// # async fn example() -> Result<(), batman_robin::Error> {
    /// let client = Client::new();
    /// client
    ///     .interface_remove(InterfaceSelector::with_name("wlan0"))
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn interface_remove(
        &self,
        interface_selector: model::InterfaceSelector,
    ) -> Result<(), Error> {
        let iface_ifindex = self
            .interface_selector_to_ifindex(interface_selector)
            .await?;

        commands::set_interface(iface_ifindex, None).await
    }

    /// Creates a new BATMAN-adv mesh interface with an optional routing algorithm.
    ///
    /// # Arguments
    /// * `mesh_if` - Name of the interface to create.
    /// * `routing_algo` - Optional routing algorithm string.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use batman_robin::Client;
    ///
    /// # async fn example() -> Result<(), batman_robin::Error> {
    /// let client = Client::new();
    /// client.mesh_create("bat0", Some("BATMAN_V")).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn mesh_create(
        &self,
        mesh_if: &str,
        routing_algo: Option<&str>,
    ) -> Result<(), Error> {
        commands::create_interface(mesh_if, routing_algo).await
    }

    /// Destroys a BATMAN-adv mesh interface selected by name or ifindex.
    ///
    /// # Arguments
    /// * `selector` - Mesh selector.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use batman_robin::{Client, MeshSelector};
    ///
    /// # async fn example() -> Result<(), batman_robin::Error> {
    /// let client = Client::new();
    /// client.mesh_delete(MeshSelector::with_name("bat0")).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn mesh_delete(&self, selector: model::MeshSelector) -> Result<(), Error> {
        let ifindex = self.selector_to_ifindex(selector).await?;
        commands::destroy_interface(ifindex).await
    }

    /// Counts the number of physical interfaces attached to the selected mesh interface.
    ///
    /// # Arguments
    /// * `selector` - Mesh selector.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use batman_robin::{Client, MeshSelector};
    ///
    /// # async fn example() -> Result<(), batman_robin::Error> {
    /// let client = Client::new();
    /// let count = client.interfaces_count(MeshSelector::with_name("bat0")).await?;
    /// println!("{count}");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn interfaces_count(&self, selector: model::MeshSelector) -> Result<u32, Error> {
        let ifindex = self.selector_to_ifindex(selector).await?;
        commands::count_interfaces(ifindex).await
    }

    /// Gets whether packet aggregation is enabled on the selected mesh interface.
    ///
    /// # Arguments
    /// * `selector` - Mesh selector.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use batman_robin::{Client, MeshSelector};
    ///
    /// # async fn example() -> Result<(), batman_robin::Error> {
    /// let client = Client::new();
    /// let enabled = client.get_aggregation(MeshSelector::with_name("bat0")).await?;
    /// println!("{enabled}");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_aggregation(&self, selector: model::MeshSelector) -> Result<bool, Error> {
        let ifindex = self.selector_to_ifindex(selector).await?;
        commands::get_aggregation(ifindex).await
    }

    /// Enables or disables packet aggregation on the selected mesh interface.
    ///
    /// # Arguments
    /// * `selector` - Mesh selector.
    /// * `val` - `true` to enable, `false` to disable.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use batman_robin::{Client, MeshSelector};
    ///
    /// # async fn example() -> Result<(), batman_robin::Error> {
    /// let client = Client::new();
    /// client
    ///     .set_aggregation(MeshSelector::with_name("bat0"), true)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_aggregation(
        &self,
        selector: model::MeshSelector,
        val: bool,
    ) -> Result<(), Error> {
        let ifindex = self.selector_to_ifindex(selector).await?;
        commands::set_aggregation(ifindex, val).await
    }

    /// Gets whether AP isolation is enabled on the selected mesh interface.
    ///
    /// # Arguments
    /// * `selector` - Mesh selector.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use batman_robin::{Client, MeshSelector};
    ///
    /// # async fn example() -> Result<(), batman_robin::Error> {
    /// let client = Client::new();
    /// let enabled = client
    ///     .get_ap_isolation(MeshSelector::with_name("bat0"))
    ///     .await?;
    /// println!("{enabled}");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_ap_isolation(&self, selector: model::MeshSelector) -> Result<bool, Error> {
        let ifindex = self.selector_to_ifindex(selector).await?;
        commands::get_ap_isolation(ifindex).await
    }

    /// Enables or disables AP isolation on the selected mesh interface.
    ///
    /// # Arguments
    /// * `selector` - Mesh selector.
    /// * `val` - `true` to enable, `false` to disable.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use batman_robin::{Client, MeshSelector};
    ///
    /// # async fn example() -> Result<(), batman_robin::Error> {
    /// let client = Client::new();
    /// client
    ///     .set_ap_isolation(MeshSelector::with_name("bat0"), true)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_ap_isolation(
        &self,
        selector: model::MeshSelector,
        val: bool,
    ) -> Result<(), Error> {
        let ifindex = self.selector_to_ifindex(selector).await?;
        commands::set_ap_isolation(ifindex, val).await
    }

    /// Gets whether bridge loop avoidance is enabled on the selected mesh interface.
    ///
    /// # Arguments
    /// * `selector` - Mesh selector.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use batman_robin::{Client, MeshSelector};
    ///
    /// # async fn example() -> Result<(), batman_robin::Error> {
    /// let client = Client::new();
    /// let enabled = client
    ///     .get_bridge_loop_avoidance(MeshSelector::with_name("bat0"))
    ///     .await?;
    /// println!("{enabled}");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_bridge_loop_avoidance(
        &self,
        selector: model::MeshSelector,
    ) -> Result<bool, Error> {
        let ifindex = self.selector_to_ifindex(selector).await?;
        commands::get_bridge_loop_avoidance(ifindex).await
    }

    /// Enables or disables bridge loop avoidance on the selected mesh interface.
    ///
    /// # Arguments
    /// * `selector` - Mesh selector.
    /// * `val` - `true` to enable, `false` to disable.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use batman_robin::{Client, MeshSelector};
    ///
    /// # async fn example() -> Result<(), batman_robin::Error> {
    /// let client = Client::new();
    /// client
    ///     .set_bridge_loop_avoidance(MeshSelector::with_name("bat0"), true)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_bridge_loop_avoidance(
        &self,
        selector: model::MeshSelector,
        val: bool,
    ) -> Result<(), Error> {
        let ifindex = self.selector_to_ifindex(selector).await?;
        commands::set_bridge_loop_avoidance(ifindex, val).await
    }

    /// Retrieves the system default routing algorithm for BATMAN-adv.
    ///
    /// # Errors
    /// Returns [`Error`] if the value cannot be retrieved from kernel state.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use batman_robin::Client;
    ///
    /// # async fn example() -> Result<(), batman_robin::Error> {
    /// let client = Client::new();
    /// let algo = client.get_default_routing_algo().await?;
    /// println!("{algo}");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_default_routing_algo(&self) -> Result<String, Error> {
        commands::get_default_routing_algo().await
    }

    /// Retrieves all active routing algorithms currently in use and their interfaces.
    ///
    /// Returns a vector of `(interface_name, algorithm_name)`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use batman_robin::Client;
    ///
    /// # async fn example() -> Result<(), batman_robin::Error> {
    /// let client = Client::new();
    /// let active = client.get_active_routing_algos().await?;
    /// for (iface, algo) in active {
    ///     println!("{iface}: {algo}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_active_routing_algos(&self) -> Result<Vec<(String, String)>, Error> {
        commands::get_active_routing_algos().await
    }

    /// Retrieves all routing algorithms available on the system.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use batman_robin::Client;
    ///
    /// # async fn example() -> Result<(), batman_robin::Error> {
    /// let client = Client::new();
    /// let available = client.get_available_routing_algos().await?;
    /// println!("{} available algos", available.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_available_routing_algos(&self) -> Result<Vec<String>, Error> {
        commands::get_available_routing_algos().await
    }

    /// Sets the system default routing algorithm.
    ///
    /// # Arguments
    /// * `algo` - Algorithm name to set as default.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use batman_robin::Client;
    ///
    /// # async fn example() -> Result<(), batman_robin::Error> {
    /// let client = Client::new();
    /// client.set_default_routing_algo("BATMAN_V").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_default_routing_algo(&self, algo: &str) -> Result<(), Error> {
        commands::set_default_routing_algo(algo).await
    }
}
