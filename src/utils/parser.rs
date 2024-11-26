use super::errors::ConfigError;
use crate::types::parsed_nodes::{ParsedClient, ParsedDrone, ParsedServer};
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

    /// Check if the network topology is valid
    fn check_topology(&self) -> Result<(), ConfigError> {
        let all_ids: HashSet<NodeId> = self
            .drones
            .iter()
            .map(|d| d.id)
            .chain(self.clients.iter().map(|c| c.id))
            .chain(self.servers.iter().map(|s| s.id))
            .collect();

        // Check that all ids are unique
        assert_eq!(
            all_ids.len(),
            self.drones.len() + self.clients.len() + self.servers.len()
        );

        // Convert drones to a lookup map for bidirectional checks
        let drone_map: HashMap<NodeId, &ParsedDrone> =
            self.drones.iter().map(|d| (d.id, d)).collect();

        // Check that connections do not contain the drone id nor are duplicated
        for drone in &self.drones {
            let mut connection_set = HashSet::new();
            for connection in &drone.connected_drone_ids {
                if *connection == drone.id
                    || !connection_set.insert(connection)
                    || !all_ids.contains(connection)
                {
                    return Err(ConfigError::InvalidDroneConnection(drone.id, *connection));
                }

                // Check bidirectionality
                if let Some(neighbor) = drone_map.get(&connection) {
                    if !neighbor.connected_drone_ids.contains(&drone.id) {
                        return Err(ConfigError::UnidirectionalConnection(drone.id, *connection));
                    }
                }
            }
        }

        // Check that connections do not contain the client id nor are duplicated
        for client in &self.clients {
            let mut connection_set = HashSet::new();
            for connection in &client.connected_drone_ids {
                if *connection == client.id
                    || !connection_set.insert(connection)
                    || !all_ids.contains(connection)
                {
                    return Err(ConfigError::InvalidClientConnection(client.id, *connection));
                }

                // Check bidirectionality
                if let Some(neighbor) = drone_map.get(&connection) {
                    if !neighbor.connected_drone_ids.contains(&client.id) {
                        return Err(ConfigError::UnidirectionalConnection(
                            client.id,
                            *connection,
                        ));
                    }
                }
            }
        }

        // Check that connections do not contain the server id nor are duplicated
        for server in &self.servers {
            let mut connection_set = HashSet::new();
            for connection in &server.connected_drone_ids {
                if *connection == server.id
                    || !connection_set.insert(connection)
                    || !all_ids.contains(connection)
                {
                    return Err(ConfigError::InvalidServerConnection(server.id, *connection));
                }

                // Check bidirectionality
                if let Some(neighbor) = drone_map.get(&connection) {
                    if !neighbor.connected_drone_ids.contains(&server.id) {
                        return Err(ConfigError::UnidirectionalConnection(
                            server.id,
                            *connection,
                        ));
                    }
                }
            }
        }

        Ok(())
    }
}
