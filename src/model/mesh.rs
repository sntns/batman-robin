use validator::Validate;

/// Selects a BATMAN-adv mesh interface either by interface name or by ifindex.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct MeshSelector {
    /// Selects the mesh interface by interface name (for example, `"bat0"`).
    pub name: Option<String>,
    /// Selects the mesh interface by Linux interface index.
    pub ifindex: Option<u32>,
}

impl MeshSelector {
    /// Creates an empty selector.
    pub fn builder() -> MeshSelectorBuilder {
        MeshSelectorBuilder::new()
    }

    /// Creates a selector from an interface name.
    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            ifindex: None,
        }
    }

    /// Creates a selector from an interface index.
    pub const fn with_ifindex(ifindex: u32) -> Self {
        Self {
            name: None,
            ifindex: Some(ifindex),
        }
    }
}

impl validator::Validate for MeshSelector {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        let mut errors = validator::ValidationErrors::new();

        if self.name.is_none() && self.ifindex.is_none() {
            errors.add(
                "mesh_selector",
                validator::ValidationError {
                    code: std::borrow::Cow::from("at_least_one_field"),
                    message: Some(std::borrow::Cow::from(
                        "At least one of 'name' or 'ifindex' must be set",
                    )),
                    params: std::collections::HashMap::new(),
                },
            );
        }

        if let Some(name) = &self.name
            && name.trim().is_empty()
        {
            errors.add(
                "name",
                validator::ValidationError {
                    code: std::borrow::Cow::from("empty_name"),
                    message: Some(std::borrow::Cow::from("Mesh selector name cannot be empty")),
                    params: std::collections::HashMap::new(),
                },
            );
        }

        if let Some(ifindex) = self.ifindex
            && ifindex == 0
        {
            errors.add(
                "ifindex",
                validator::ValidationError {
                    code: std::borrow::Cow::from("invalid_ifindex"),
                    message: Some(std::borrow::Cow::from(
                        "Mesh selector ifindex must be greater than 0",
                    )),
                    params: std::collections::HashMap::new(),
                },
            );
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[derive(Debug, Default)]
pub struct MeshSelectorBuilder {
    selector: MeshSelector,
}

impl MeshSelectorBuilder {
    pub fn new() -> Self {
        Self {
            selector: MeshSelector::default(),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.selector.name = Some(name.into());
        self
    }

    pub fn with_ifindex(mut self, ifindex: u32) -> Self {
        self.selector.ifindex = Some(ifindex);
        self
    }

    pub fn build(self) -> Result<MeshSelector, validator::ValidationErrors> {
        self.selector.validate()?;
        Ok(self.selector)
    }
}
