use super::Protocol;

// TODO: Protocols must be stored somewhere else. Not in memory.
/// Manages a collection of protocols.
pub struct ProtocolManager {
    protocols: Vec<Protocol>,
}

// TODO: Possible errors should be handled here.
impl ProtocolManager {
    /// Creates a new `ProtocolManager`.
    ///
    /// # Returns
    ///
    /// A new instance of `ProtocolManager` with an empty list of protocols.
    pub fn new() -> Self {
        Self {
            protocols: Vec::new(),
        }
    }

    /// Creates a new protocol and adds it to the manager.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the protocol.
    /// * `description` - A brief description of the protocol.
    ///
    /// # Returns
    ///
    /// The newly created `Protocol`.
    pub fn create_protocol(&mut self, name: String, description: String) -> Protocol {
        let protocol = Protocol { name, description };
        self.protocols.push(protocol.clone());
        protocol
    }

    /// Retrieves a protocol by its name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the protocol to retrieve.
    ///
    /// # Returns
    ///
    /// An `Option` containing a reference to the `Protocol` if found, or `None` if not found.
    pub fn get_protocol(&self, name: &str) -> Option<&Protocol> {
        self.protocols.iter().find(|&p| p.name == name)
    }

    /// Updates the description of an existing protocol.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the protocol to update.
    /// * `new_description` - The new description of the protocol.
    ///
    /// # Returns
    ///
    /// An `Option` containing a reference to the updated `Protocol` if found, or `None` if not found.
    pub fn update_protocol(&mut self, name: &str, new_description: String) -> Option<&Protocol> {
        if let Some(protocol) = self.protocols.iter_mut().find(|p| p.name == name) {
            protocol.description = new_description;
            return Some(protocol);
        }
        None
    }

    /// Deletes a protocol by its name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the protocol to delete.
    ///
    /// # Returns
    ///
    /// `true` if the protocol was found and deleted, `false` otherwise.
    pub fn delete_protocol(&mut self, name: &str) -> bool {
        let len_before = self.protocols.len();
        self.protocols.retain(|p| p.name != name);
        len_before != self.protocols.len()
    }
}
