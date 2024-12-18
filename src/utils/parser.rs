use super::errors::ConfigError;
use crate::{
    parsed_nodes::Node,
    types::parsed_nodes::{ParsedClient, ParsedDrone, ParsedServer},
};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    fs,
};
use wg_internal::network::NodeId;

#[derive(Debug, Deserialize)]
pub struct Parser {
    pub drones: Vec<ParsedDrone>,
    pub clients: Vec<ParsedClient>,
    pub servers: Vec<ParsedServer>,
}

impl Parser {
    /// Create a new configuration
    /// # Errors
    /// Returns an error if the configuration file cannot be read or the configuration is invalid
    pub fn new(path: Option<&str>) -> Result<Self, ConfigError> {
        let mut config = Parser {
            drones: Vec::new(),
            clients: Vec::new(),
            servers: Vec::new(),
        };

        if let Some(path) = path {
            config.parse_config_file(path)?;
        }

        Ok(config)
    }

    /// Parse the configuration file and update the configuration
    /// # Errors
    /// Returns an error if the file cannot be read or the configuration is invalid
    pub fn parse_config_file(&mut self, path: &str) -> Result<(), ConfigError> {
        let config_data =
            fs::read_to_string(path).map_err(|_| ConfigError::FileReadError(path.to_string()))?;
        let config: Parser =
            toml::from_str(&config_data).map_err(|_| ConfigError::ParseError(path.to_string()))?;

        self.drones = config.drones;
        self.clients = config.clients;
        self.servers = config.servers;

        self.check_topology()
    }

    fn generic_check_topology<T: Node>(
        nodes: &[T],
        all_ids: &HashSet<NodeId>,
        drone_map: &HashMap<NodeId, &dyn Node>,
    ) -> Result<(), ConfigError> {
        // Check that connections do not contain the node id nor are duplicated
        for node in nodes {
            let mut connection_set = HashSet::new();
            for connection in node.connected_drone_ids() {
                if *connection == node.id()
                    || !connection_set.insert(connection)
                    || !all_ids.contains(connection)
                {
                    return Err(ConfigError::InvalidNodeConnection(node.id(), *connection));
                }

                // Check bidirectionality
                if let Some(neighbor) = drone_map.get(connection) {
                    if !neighbor.connected_drone_ids().contains(&node.id()) {
                        return Err(ConfigError::UnidirectionalConnection(
                            node.id(),
                            *connection,
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    fn check_topology(&self) -> Result<(), ConfigError> {
        //TODO check servers are connected to at least 2 drones
        //TODO check clients are connected to at most 2 drones

        let all_ids: HashSet<NodeId> = self
            .drones
            .iter()
            .map(Node::id)
            .chain(self.clients.iter().map(Node::id))
            .chain(self.servers.iter().map(Node::id))
            .collect();

        // Check that all ids are unique
        if all_ids.len() != self.drones.len() + self.clients.len() + self.servers.len() {
            return Err(ConfigError::DuplicatedNodeId);
        }

        // Convert nodes to a lookup map for bidirectional checks
        let node_map: HashMap<NodeId, &dyn Node> = self
            .drones
            .iter()
            .map(|d| (d.id(), d as &dyn Node))
            .chain(self.clients.iter().map(|c| (c.id(), c as &dyn Node)))
            .chain(self.servers.iter().map(|s| (s.id(), s as &dyn Node)))
            .collect();

        Parser::generic_check_topology(&self.drones, &all_ids, &node_map)?;
        Parser::generic_check_topology(&self.clients, &all_ids, &node_map)?;
        Parser::generic_check_topology(&self.servers, &all_ids, &node_map)?;

        Ok(())
    }
}
