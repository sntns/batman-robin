use neli::genl::{AttrTypeBuilder, NlattrBuilder};
use neli::types::{Buffer, GenlBuffer};

use crate::error::Error;
use crate::model::{AttrValueForSend, Attribute};

/// Builder for Generic Netlink attributes.
///
/// Simplifies creating a `GenlBuffer` containing multiple attributes to send
/// in a Generic Netlink message to BATMAN-adv.
///
/// This is a convenience wrapper over `neli`’s `GenlBuffer`, `NlattrBuilder`,
/// and `AttrTypeBuilder`, handling conversion from Rust types to netlink payloads.
pub(crate) struct GenlAttrBuilder {
    buf: GenlBuffer<u16, Buffer>,
}

impl GenlAttrBuilder {
    /// Creates a new empty attribute builder.
    ///
    /// # Returns
    /// A `GenlAttrBuilder` ready to have attributes added.
    pub(crate) fn new() -> Self {
        Self {
            buf: GenlBuffer::new(),
        }
    }

    /// Adds an attribute to the builder.
    ///
    /// # Parameters
    /// - `attr`: The `Attribute` enum specifying which BATMAN-adv attribute to set.
    /// - `value`: The value to associate with the attribute, as `AttrValueForSend`.
    ///
    /// # Returns
    /// - `Ok(())` on success.
    /// - `Err(Error)` if building the netlink attribute fails.
    pub(crate) fn add(&mut self, attr: Attribute, value: AttrValueForSend) -> Result<(), Error> {
        let attr_type = AttrTypeBuilder::default()
            .nla_type(attr.into())
            .build()
            .map_err(|e| Error::Netlink(format!("Failed to build AttrType: {:?}", e)))?;

        let attr_payload = match value {
            AttrValueForSend::String(s) => {
                let mut b = s.into_bytes();
                b.push(0);
                b
            }
            AttrValueForSend::Bytes(b) => b,
            AttrValueForSend::U32(v) => v.to_le_bytes().to_vec(),
            AttrValueForSend::U16(v) => v.to_le_bytes().to_vec(),
            AttrValueForSend::U8(v) => vec![v],
        };

        let attr = NlattrBuilder::default()
            .nla_type(attr_type)
            .nla_payload(attr_payload)
            .build()
            .map_err(|e| Error::Netlink(format!("Failed to build Nlattr: {:?}", e)))?;

        self.buf.push(attr);
        Ok(())
    }

    /// Consumes the builder and returns the final `GenlBuffer`.
    ///
    /// # Returns
    /// A `GenlBuffer<u16, Buffer>` containing all added attributes, ready
    /// to be sent in a Generic Netlink message.
    pub(crate) fn build(self) -> GenlBuffer<u16, Buffer> {
        self.buf
    }
}
