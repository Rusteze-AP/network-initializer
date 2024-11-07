mod errors;

use crate::errors::ConfigError;
use serde::Deserialize;
use std::{collections::HashSet, fs};

pub type NodeId = u64;

#[derive(Debug, Deserialize)]
pub struct Drone {
    id: NodeId,
    connected_drone_ids: Vec<NodeId>,
    pdr: f64,
}

#[derive(Debug, Deserialize)]
pub struct Client {
    id: NodeId,
    connected_drone_ids: Vec<NodeId>,
}

#[derive(Debug, Deserialize)]
pub struct Server {
    id: NodeId,
    connected_drone_ids: Vec<NodeId>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub drones: Vec<Drone>,
    pub clients: Vec<Client>,
    pub servers: Vec<Server>,
}

impl Config {
    /// Create a new configuration
    /// # Errors
    /// Returns an error if the configuration file cannot be read or the configuration is invalid
    pub fn new(path: Option<&str>) -> Result<Self, ConfigError> {
        let mut config = Config {
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
        let config: Config =
            toml::from_str(&config_data).map_err(|_| ConfigError::ParseError(path.to_string()))?;

        self.drones = config.drones;
        self.clients = config.clients;
        self.servers = config.servers;

        self.check_topology()
    }

    /// Check if the network topology is valid
    fn check_topology(&self) -> Result<(), ConfigError> {
        let drone_ids: HashSet<NodeId> = self.drones.iter().map(|d| d.id).collect();

        // check that connections do not contain the drone id nor are duplicated
        for drone in &self.drones {
            let mut connection_set = HashSet::new();
            for connection in &drone.connected_drone_ids {
                if *connection == drone.id
                    || !connection_set.insert(connection)
                    || !drone_ids.contains(connection)
                {
                    return Err(ConfigError::InvalidDroneConnection(drone.id, *connection));
                }
            }
        }

        // check that connections do not contain the client id nor are duplicated
        for client in &self.clients {
            let mut connection_set = HashSet::new();
            for connection in &client.connected_drone_ids {
                if *connection == client.id
                    || !connection_set.insert(connection)
                    || !drone_ids.contains(connection)
                {
                    return Err(ConfigError::InvalidClientConnection(client.id, *connection));
                }
            }
        }

        // check that connections do not contain the server id nor are duplicated
        for server in &self.servers {
            let mut connection_set = HashSet::new();
            for connection in &server.connected_drone_ids {
                if *connection == server.id
                    || !connection_set.insert(connection)
                    || !drone_ids.contains(connection)
                {
                    return Err(ConfigError::InvalidServerConnection(server.id, *connection));
                }
            }
        }

        Ok(())
    }
}
