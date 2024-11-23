mod types;
mod utils;

use crossbeam::channel::{unbounded, Receiver, Sender};
use drone::drone::RustezeDrone;
use std::collections::HashMap;
use std::fmt::Debug;
use std::thread;
use std::time::Duration;
use types::channel::Channel;
use types::nodes::{Client, ClientTrait, Server, ServerTrait};
use types::parsed_nodes::{Initializable, NodeId};
use utils::errors::ConfigError;
use utils::parser::Parser;
use wg_internal::packet::Packet;

#[derive(Debug)]
pub struct NetworkInitializer {
    parser: Parser,
}

impl NetworkInitializer {
    /// Create a new configuration
    /// # Errors
    /// Returns an error if parser encounters an error
    pub fn new(path: Option<&str>) -> Result<Self, ConfigError> {
        let parser = Parser::new(path)?;

        Ok(NetworkInitializer { parser })
    }

    fn create_channels(&self) -> HashMap<NodeId, Channel> {
        let mut channel_map = HashMap::new();
        for drone in &self.parser.drones {
            let (tx, rx) = unbounded();
            let channel = Channel::new(tx, rx);
            channel_map.insert(drone.id, channel);
        }
        for client in &self.parser.clients {
            let (tx, rx) = unbounded();
            let channel = Channel::new(tx, rx);
            channel_map.insert(client.id, channel);
        }
        for server in &self.parser.servers {
            let (tx, rx) = unbounded();
            let channel = Channel::new(tx, rx);
            channel_map.insert(server.id, channel);
        }

        channel_map
    }

    fn initialize_entities<T, F, O>(
        nodes: &[T],
        channel_map: &HashMap<NodeId, Channel>,
        create_entity: F,
    ) -> Vec<O>
    where
        T: Initializable,
        F: Fn(&T, HashMap<NodeId, Sender<Packet>>, Receiver<Packet>) -> O,
    {
        nodes
            .iter()
            .map(|node| {
                let mut senders = HashMap::new();

                for neighbor_id in node.connected_drone_ids() {
                    if let Some(channel) = channel_map.get(neighbor_id) {
                        senders.insert(*neighbor_id, channel.sender.clone());
                    }
                }

                let receiver = channel_map
                    .get(node.id())
                    .expect("Receiver must exist")
                    .receiver
                    .clone();

                create_entity(node, senders, receiver)
            })
            .collect()
    }

    fn initialize_network(&self) -> (Vec<RustezeDrone>, Vec<Client>, Vec<Server>) {
        let channel_map = self.create_channels();

        let initialized_drones = Self::initialize_entities(
            &self.parser.drones,
            &channel_map,
            |drone, senders, receiver| RustezeDrone::new(drone.id, drone.pdr, receiver, senders),
        );

        let initialized_clients = Self::initialize_entities(
            &self.parser.clients,
            &channel_map,
            |client, senders, receiver| Client::new(client.id, receiver, senders),
        );

        let initialized_servers = Self::initialize_entities(
            &self.parser.servers,
            &channel_map,
            |server, senders, receiver| Server::new(server.id, receiver, senders),
        );

        (initialized_drones, initialized_clients, initialized_servers)
    }

    pub fn run_simulation(&self) {
        let (drones, clients, servers) = self.initialize_network();

        // Start drones
        for drone in drones {
            thread::spawn(move || {
                drone.run();
            });
        }

        // Start clients
        for client in clients {
            thread::spawn(move || {
                client.run();
            });
        }

        // Start servers
        for server in servers {
            thread::spawn(move || {
                server.run();
            });
        }

        // Start the simulation
        loop {
            thread::sleep(Duration::from_secs(1));
        }
    }
}
