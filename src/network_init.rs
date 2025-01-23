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
enum State {
    Instantiated,
    Initialized,
    Running,
}

#[derive(Debug)]
pub struct NetworkInitializer {
    state: State,
    steps_done: u8,

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

    fn switch_state(&mut self) {
        self.steps_done += 1;
        if self.steps_done == 3 {
            self.state = State::Initialized;
        }
    }

    #[must_use]
    pub fn get_controller_recv(&mut self) -> Receiver<DroneEvent> {
        self.switch_state();
        self.node_event.receiver.clone()
    }

    #[must_use]
    pub fn get_controller_senders(&mut self) -> HashMap<NodeId, Sender<DroneCommand>> {
        self.switch_state();
        self.drone_command_map
            .iter()
            .map(|(id, channel)| (*id, channel.sender.clone()))
            .collect()
    }

    /// Get the channels from the network initializer
    /// # Note
    /// This function should only be called once, when running the simulation the channels are consumed
    #[must_use]
    pub fn get_channels(&mut self) -> HashMap<NodeId, Channel<Packet>> {
        self.switch_state();
        self.channel_map.clone()
    }
}

impl NetworkInitializer {
    /// Create a new configuration
    /// # Errors
    /// Returns an error if parser encounters an error
    pub fn new(path: Option<&str>) -> Result<Self, ConfigError> {
        let parser = Parser::new(path)?;

        let mut net_init = NetworkInitializer {
            state: State::Instantiated,
            steps_done: 0,
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
            self.channel_map.insert(drone.id, Channel::default());
            self.drone_command_map.insert(drone.id, Channel::default());
        }
        for client in &self.parser.clients {
            self.channel_map.insert(client.id, Channel::default());
            self.drone_command_map.insert(client.id, Channel::default());
        }
        for server in &self.parser.servers {
            self.channel_map.insert(server.id, Channel::default());
            self.drone_command_map.insert(server.id, Channel::default());
        }

        self.node_event = Channel::default();
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

    fn initialize_network(
        &mut self,
        rt: &Runtime,
    ) -> (Vec<RustezeDrone>, Vec<Client>, Vec<Server>) {
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
                rt.block_on(Client::new(
                    client.id,
                    command_send,
                    command_recv,
                    receiver,
                    senders,
                ))
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

        self.channel_map.clear();
        (initialized_drones, initialized_clients, initialized_servers)
    }

    /// Run the simulation
    /// # Errors
    /// Returns an error if the state is not initialized (`get_channels()`, `get_controller_recv()`, `get_controller_senders()` must be called first)
    /// # Panics
    /// Panics if the tokio runtime fails to start
    pub fn run_simulation(&mut self) -> Result<(), String> {
        let res: Result<(), String> = match self.state {
            State::Initialized => {
                self.state = State::Running;
                Ok(())
            }
            _ => Err("run_simulation() can only be called when initialized".into()),
        };
        res.as_ref()?;

        let rt = Runtime::new().expect("Failed to create Tokio runtime");

        let (drones, clients, servers) = self.initialize_network(&rt);
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
                        client.with_all();
                        client.run().await;
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

            // Send crash message to all nodes
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

        Ok(())
    }

    pub fn stop_simulation(&mut self) {
        todo!()
    }
}
