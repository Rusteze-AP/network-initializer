use crate::types;
use crate::utils;

use crossbeam::channel::{unbounded, Receiver, Sender};
use rusteze_drone::RustezeDrone;
use std::collections::HashMap;
use std::fmt::Debug;
use std::thread::{self, JoinHandle};
use types::channel::Channel;
use types::nodes::{Client, ClientTrait, Server, ServerTrait};
use types::parsed_nodes::Initializable;
use types::parsed_nodes::{ParsedClient, ParsedDrone, ParsedServer};
use utils::errors::ConfigError;
use utils::parser::Parser;
use wg_internal::controller::{DroneCommand, NodeEvent};
use wg_internal::drone::{Drone, DroneOptions};
use wg_internal::network::NodeId;
use wg_internal::packet::Packet;

#[derive(Debug)]
pub struct NetworkInitializer {
    parser: Parser,
    // normal communication channels
    channel_map: HashMap<NodeId, Channel<Packet>>,
    // channels from controller to drones
    drone_command_map: HashMap<NodeId, Channel<DroneCommand>>,
    // channel from drones to controller
    node_event: Channel<NodeEvent>,
    node_handlers: HashMap<NodeId, JoinHandle<()>>,
}

impl NetworkInitializer {
    pub fn set_path(&mut self, path: Option<&str>) -> Result<(), ConfigError> {
        self.parser = Parser::new(path)?;
        Ok(())
    }

    pub fn get_nodes(&self) -> (&Vec<ParsedDrone>, &Vec<ParsedClient>, &Vec<ParsedServer>) {
        (
            &self.parser.drones,
            &self.parser.clients,
            &self.parser.servers,
        )
    }
}

impl NetworkInitializer {
    /// Create a new configuration
    /// # Errors
    /// Returns an error if parser encounters an error
    pub fn new(path: Option<&str>) -> Result<Self, ConfigError> {
        let parser = Parser::new(path)?;

        Ok(NetworkInitializer {
            parser,
            channel_map: HashMap::new(),
            drone_command_map: HashMap::new(),
            node_event: Channel::new(unbounded().0, unbounded().1),
            node_handlers: HashMap::new(),
        })
    }

    fn create_channels(&mut self) {
        for drone in &self.parser.drones {
            let (tx, rx) = unbounded();
            let channel: Channel<Packet> = Channel::new(tx, rx);
            self.channel_map.insert(drone.id, channel);

            let (tx, rx) = unbounded();
            let command_channel: Channel<DroneCommand> = Channel::new(tx, rx);
            self.drone_command_map.insert(drone.id, command_channel);
        }
        for client in &self.parser.clients {
            let (tx, rx) = unbounded();
            let channel = Channel::new(tx, rx);
            self.channel_map.insert(client.id, channel);

            let (tx, rx) = unbounded();
            let command_channel: Channel<DroneCommand> = Channel::new(tx, rx);
            self.drone_command_map.insert(client.id, command_channel);
        }
        for server in &self.parser.servers {
            let (tx, rx) = unbounded();
            let channel = Channel::new(tx, rx);
            self.channel_map.insert(server.id, channel);

            let (tx, rx) = unbounded();
            let command_channel: Channel<DroneCommand> = Channel::new(tx, rx);
            self.drone_command_map.insert(server.id, command_channel);
        }

        let (tx, rx) = unbounded();
        let channel: Channel<NodeEvent> = Channel::new(tx, rx);
        self.node_event = channel;
    }

    fn initialize_entities<T, F, O>(
        nodes: &[T],
        channel_map: &HashMap<NodeId, Channel<Packet>>,
        channel_command_map: &HashMap<NodeId, Channel<DroneCommand>>,
        node_event: &Channel<NodeEvent>,
        create_entity: F,
    ) -> Vec<O>
    where
        T: Initializable,
        F: Fn(
            &T,
            Sender<NodeEvent>,
            Receiver<DroneCommand>,
            HashMap<NodeId, Sender<Packet>>,
            Receiver<Packet>,
        ) -> O,
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
                let command_recv = channel_command_map
                    .get(node.id())
                    .expect("Command receiver must exist")
                    .receiver
                    .clone();
                let command_send = node_event.sender.clone();

                create_entity(node, command_send, command_recv, senders, receiver)
            })
            .collect()
    }

    fn initialize_network(&mut self) -> (Vec<RustezeDrone>, Vec<Client>, Vec<Server>) {
        self.create_channels();

        let initialized_drones = Self::initialize_entities(
            &self.parser.drones,
            &self.channel_map,
            &self.drone_command_map,
            &self.node_event,
            |drone, command_send, command_recv, senders, receiver| {
                RustezeDrone::new(DroneOptions {
                    id: drone.id,
                    controller_send: command_send,
                    controller_recv: command_recv,
                    packet_send: senders,
                    packet_recv: receiver,
                    pdr: drone.pdr,
                })
            },
        );

        let initialized_clients = Self::initialize_entities(
            &self.parser.clients,
            &self.channel_map,
            &self.drone_command_map,
            &self.node_event,
            |client, command_send, command_recv, senders, receiver| {
                Client::new(client.id, receiver, senders)
            },
        );

        let initialized_servers = Self::initialize_entities(
            &self.parser.servers,
            &self.channel_map,
            &self.drone_command_map,
            &self.node_event,
            |server, command_send, command_recv, senders, receiver| {
                Server::new(server.id, receiver, senders)
            },
        );

        (initialized_drones, initialized_clients, initialized_servers)
    }

    pub fn run_simulation(&mut self) {
        let (drones, clients, servers) = self.initialize_network();

        // Start drones
        for mut drone in drones {
            self.node_handlers.insert(
                drone.get_id(),
                thread::spawn(move || {
                    drone.run();
                }),
            );
        }

        // Start clients
        for client in clients {
            self.node_handlers.insert(
                client.get_id(),
                thread::spawn(move || {
                    client.run();
                }),
            );
        }

        // Start servers
        for server in servers {
            self.node_handlers.insert(
                server.get_id(),
                thread::spawn(move || {
                    server.run();
                }),
            );
        }

        // Wait for all threads to finish
        for (_, handler) in self.node_handlers.drain() {
            handler.join().unwrap();
        }
    }
}
