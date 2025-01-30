mod getters;
mod net_utils;

use crate::create_drone_factories;
use crate::parsed_nodes::ParsedClient;
use crate::parsed_nodes::ParsedDrone;
use crate::parsed_nodes::ParsedServer;
use crate::types;
use crate::utils;

use client::Client as ClientVideo;
// use client_audio::ClientAudio;
use crossbeam::channel::{unbounded, Receiver, Sender};
use net_utils::BoxDrone;
use packet_forge::ClientT;
use server::Server;
use std::collections::HashMap;
use std::fmt::Debug;
use std::thread::{self, JoinHandle};
use types::channel::Channel;
use types::parsed_nodes::Initializable;
use utils::errors::ConfigError;
use utils::parser::Parser;
use wg_internal::controller::{DroneCommand, DroneEvent};
use wg_internal::drone::Drone;
use wg_internal::network::NodeId;
use wg_internal::packet::Packet;

use rusteze_drone::RustezeDrone;

use rustbusters_drone::RustBustersDrone;
// use dr_ones::Drone as dr_ones_drone;
use ap2024_unitn_cppenjoyers_drone::CppEnjoyersDrone;
use lockheedrustin_drone::LockheedRustin;
use null_pointer_drone::MyDrone as NullPointerDrone;
use rust_do_it::RustDoIt;
use rust_roveri::RustRoveri;
use rusty_drones::RustyDrone;
use skylink::SkyLinkDrone;
use wg_2024_rust::drone::RustDrone;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum DroneType {
    RustezeDrone,
    RustBustersDrone,
    RustDrone,
    RustRoveri,
    RustDoIt,
    LockheedRustin,
    CppEnjoyersDrone,
    SkyLinkDrone,
    RustyDrone,
    NullPointerDrone,
}

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
        create_entity: &[F],
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
            .enumerate()
            .map(|(index, node)| {
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

                // Use the current new method to create the entity
                let create_entity = &create_entity[index % create_entity.len()];

                create_entity(node, command_send, command_recv, senders, receiver)
            })
            .collect()
    }

    fn initialize_network(
        &mut self,
        selected_drones: Option<Vec<DroneType>>, // Use the enum for selecting drones
    ) -> (Vec<Box<dyn Drone>>, Vec<ClientVideo>, Vec<Server>) {
        // Use the macro to generate factories mapped to DroneType
        let drone_factories = create_drone_factories!(
            RustezeDrone,
            RustBustersDrone,
            RustDrone,
            RustRoveri,
            RustDoIt,
            LockheedRustin,
            CppEnjoyersDrone,
            SkyLinkDrone,
            RustyDrone,
            NullPointerDrone
        );

        // Filter factories based on the selected drones
        let filtered_factories: Vec<BoxDrone> = if let Some(selected) = selected_drones {
            drone_factories
                .into_iter()
                .filter(|(drone_type, _)| selected.contains(drone_type))
                .map(|(_, factory)| factory)
                .collect()
        } else {
            drone_factories
                .into_iter()
                .map(|(_, factory)| factory)
                .collect() // Use all factories if no selection is provided
        };

        let initialized_drones = Self::initialize_entities(
            &self.parser.drones,
            &self.channel_map,
            &self.drone_command_map,
            &self.node_event,
            &filtered_factories,
        );

        let initialized_clients = Self::initialize_entities(
            &self.parser.clients,
            &self.channel_map,
            &self.drone_command_map,
            &self.node_event,
            &[
                |client: &ParsedClient, command_send, command_recv, senders, receiver| {
                    ClientVideo::new(
                        client.id,
                        command_send,
                        command_recv,
                        receiver,
                        senders,
                        "./initialization_files/client_video/",
                    )
                },
            ],
        );

        let initialized_servers = Self::initialize_entities(
            &self.parser.servers,
            &self.channel_map,
            &self.drone_command_map,
            &self.node_event,
            &[
                |server: &ParsedServer, command_send, command_recv, senders, receiver| {
                    Server::new(server.id, command_send, command_recv, receiver, senders)
                },
            ],
        );

        self.channel_map.clear();
        (initialized_drones, initialized_clients, initialized_servers)
    }

    /// Run the simulation
    /// ### Arguments
    /// - `selected_drones`: Vector of `DroneType`. If `None` uses all drones.
    /// ### Errors
    /// Returns an error if the state is not initialized (`get_channels()`, `get_controller_recv()`, `get_controller_senders()` must be called first)
    /// ### Panics
    /// Panics if the tokio runtime fails to start
    pub fn run_simulation(
        &mut self,
        selected_drones: Option<Vec<DroneType>>,
    ) -> Result<(), String> {
        let res: Result<(), String> = match self.state {
            State::Initialized => {
                self.state = State::Running;
                Ok(())
            }
            _ => Err("run_simulation() can only be called when initialized".into()),
        };
        res.as_ref()?;

        let (drones, clients, servers) = self.initialize_network(selected_drones);
        let mut node_handlers: HashMap<NodeId, JoinHandle<()>> = HashMap::new();

        for (i, mut drone) in drones.into_iter().enumerate() {
            node_handlers.insert(
                self.parser.drones[i].id, // Needed because drones do not implement get_id method
                thread::spawn(move || {
                    drone.run();
                }),
            );
        }

        for client in clients {
            let client_id = client.get_id();
            node_handlers.insert(
                client_id,
                thread::spawn(move || {
                    // let rt = Runtime::new().expect("Failed to create Tokio runtime");
                    // rt.block_on(async {
                    //     // client.with_all();
                    //     let _res = client.run().await;
                    // });
                    client.run();
                }),
            );
        }

        for mut server in servers {
            node_handlers.insert(
                server.get_id(),
                thread::spawn(move || {
                    server.run("./initialization_files/server");
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
