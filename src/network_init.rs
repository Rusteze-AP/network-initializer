use crate::types;
use crate::utils;

use client::Client;
use crossbeam::channel::{unbounded, Receiver, Sender};
use rusteze_drone::RustezeDrone;
use server::Server;
use std::collections::HashMap;
use std::fmt::Debug;
use std::thread::{self, JoinHandle};
use tokio::runtime::Runtime;
use types::channel::Channel;
use types::parsed_nodes::{Initializable, ParsedClient, ParsedDrone, ParsedServer};
use utils::errors::ConfigError;
use utils::parser::Parser;
use wg_internal::controller::{DroneCommand, DroneEvent};
use wg_internal::drone::Drone;
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
    node_event: Channel<DroneEvent>,
}

impl NetworkInitializer {
    /// Set the path of the configuration file
    /// # Errors
    /// Returns an error if the parser encounters an error
    pub fn set_path(&mut self, path: Option<&str>) -> Result<(), ConfigError> {
        self.parser = Parser::new(path)?;
        Ok(())
    }

    #[must_use]
    pub fn get_nodes(&self) -> (&Vec<ParsedDrone>, &Vec<ParsedClient>, &Vec<ParsedServer>) {
        (
            &self.parser.drones,
            &self.parser.clients,
            &self.parser.servers,
        )
    }

    #[must_use]
    pub fn get_controller_recv(&self) -> Receiver<DroneEvent> {
        self.node_event.receiver.clone()
    }

    #[must_use]
    pub fn get_controller_senders(&self) -> HashMap<NodeId, Sender<DroneCommand>> {
        self.drone_command_map
            .iter()
            .map(|(id, channel)| (*id, channel.sender.clone()))
            .collect()
    }
}

impl NetworkInitializer {
    /// Create a new configuration
    /// # Errors
    /// Returns an error if parser encounters an error
    pub fn new(path: Option<&str>) -> Result<Self, ConfigError> {
        let parser = Parser::new(path)?;

        let mut net_init = NetworkInitializer {
            parser,
            channel_map: HashMap::new(),
            drone_command_map: HashMap::new(),
            node_event: Channel::new(unbounded().0, unbounded().1),
        };

        net_init.create_channels();
        Ok(net_init)
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
        let channel: Channel<DroneEvent> = Channel::new(tx, rx);
        self.node_event = channel;
    }

    fn initialize_entities<T, F, O>(
        nodes: &[T],
        channel_map: &HashMap<NodeId, Channel<Packet>>,
        channel_command_map: &HashMap<NodeId, Channel<DroneCommand>>,
        node_event: &Channel<DroneEvent>,
        create_entity: F,
    ) -> Vec<O>
    where
        T: Initializable,
        F: Fn(
            &T,
            Sender<DroneEvent>,
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
        let initialized_drones = Self::initialize_entities(
            &self.parser.drones,
            &self.channel_map,
            &self.drone_command_map,
            &self.node_event,
            |drone, command_send, command_recv, senders, receiver| {
                RustezeDrone::new(
                    drone.id,
                    command_send,
                    command_recv,
                    receiver,
                    senders,
                    drone.pdr,
                )
            },
        );

        let initialized_clients = Self::initialize_entities(
            &self.parser.clients,
            &self.channel_map,
            &self.drone_command_map,
            &self.node_event,
            |client, command_send, command_recv, senders, receiver| {
                Client::new(client.id, command_send, command_recv, receiver, senders)
            },
        );

        let initialized_servers = Self::initialize_entities(
            &self.parser.servers,
            &self.channel_map,
            &self.drone_command_map,
            &self.node_event,
            |server, command_send, command_recv, senders, receiver| {
                Server::new(server.id, command_send, command_recv, receiver, senders)
            },
        );

        (initialized_drones, initialized_clients, initialized_servers)
    }

    pub fn run_simulation(&mut self) {
        let (drones, clients, servers) = self.initialize_network();
        // Create a shutdown signal that can be shared across threads
        let mut node_handlers: HashMap<NodeId, JoinHandle<()>> = HashMap::new();

        for mut drone in drones {
            node_handlers.insert(
                drone.get_id(),
                thread::spawn(move || {
                    // drone.with_all();
                    drone.run();
                }),
            );
        }

        for client in clients {
            let client_id = client.get_id();
            node_handlers.insert(
                client_id,
                thread::spawn(move || {
                    let rt = Runtime::new().expect("Failed to create Tokio runtime");
                    rt.block_on(async {
                        if let Err(e) = client.run().await {
                            eprintln!("Error running client {client_id}: {e:?}");
                        }
                    });
                }),
            );
        }

        for mut server in servers {
            node_handlers.insert(
                server.get_id(),
                thread::spawn(move || {
                    server.run();
                }),
            );
        }

        // Set up Ctrl+C handler
        let _command_senders = self.get_controller_senders();
        ctrlc::set_handler(move || {
            println!("Received Ctrl+C, shutting down...");
            std::process::exit(0);

            // Send crash message to all drones
            // for (id, sender) in &command_senders {
            //     sender.send(DroneCommand::Crash).unwrap();
            //     println!("Sent crash command to node {}", id);
            // }
        })
        .expect("Error setting Ctrl+C handler");

        for (id, handler) in node_handlers.drain() {
            match handler.join() {
                Ok(()) => {
                    println!("Node {id} shut down successfully");
                }
                Err(err) => {
                    eprintln!("Thread for node {id} panicked: {err:?}");
                }
            }
        }
    }

    pub fn stop_simulation(&mut self) {
        todo!()
    }
}
