mod getters;
mod net_utils;

use crate::create_drone_factories;
use crate::parsed_nodes::ParsedClient;
use crate::parsed_nodes::ParsedDrone;
use crate::parsed_nodes::ParsedServer;
use crate::types;
use crate::utils;

use client::Client as ClientVideo;
use client_audio::ClientAudio;
use crossbeam::channel::{unbounded, Receiver, Sender};
use net_utils::BoxClient;
use net_utils::BoxDrone;
use packet_forge::ClientT;
use packet_forge::ClientType;
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

// use rusteze_drone::RustezeDrone;

use rustbusters_drone::RustBustersDrone;
use dr_ones::Drone as DrOnes;
use ap2024_unitn_cppenjoyers_drone::CppEnjoyersDrone;
use lockheedrustin_drone::LockheedRustin;
use null_pointer_drone::MyDrone as NullPointerDrone;
use rust_do_it::RustDoIt;
use rust_roveri::RustRoveri;
use rusty_drones::RustyDrone;
use skylink::SkyLinkDrone;
use wg_2024_rust::drone::RustDrone;

type GenericDrone = Box<dyn Drone>;
type GenericClient = Box<dyn ClientT>;

const CLIENT_AUDIO_CONFIGURATIONS_NUM: usize = 1;
const SERVER_CONFIGURATIONS_NUM: usize = 1;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum DroneType {
    // RustezeDrone,
    DrOnes,
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

    fn filter_nodes<T: PartialEq, G>(
        selected_nodes: Option<Vec<T>>,
        node_factories: Vec<(T, G)>,
    ) -> Vec<G> {
        // Filter factories based on the selected drones
        if let Some(selected) = selected_nodes {
            node_factories
                .into_iter()
                .filter(|(drone_type, _)| selected.contains(drone_type))
                .map(|(_, factory)| factory)
                .collect()
        } else {
            node_factories
                .into_iter()
                .map(|(_, factory)| factory)
                .collect() // Use all factories if no selection is provided
        }
    }

    /// Returns all the instances of the needed nodes
    /// ### Arguments
    /// - `selected_drones`: if None uses all drones otherwise uses only the selected ones
    /// - `selected_clients`: if None uses all clients otherwise uses only the selected ones
    fn initialize_network(
        &mut self,
        selected_drones: Option<Vec<DroneType>>,
        selected_clients: Option<Vec<ClientType>>,
    ) -> (Vec<GenericDrone>, Vec<GenericClient>, Vec<Server>) {
        // Use the macro to generate factories mapped to DroneType
        let drone_factories: Vec<(DroneType, BoxDrone)> = create_drone_factories!(
            // RustezeDrone,
            DrOnes,
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

        let client_factories: Vec<(ClientType, BoxClient)> = vec![
            (
                ClientType::Song,
                Box::new(
                    |client: &ParsedClient,
                     command_send: Sender<DroneEvent>,
                     command_recv: Receiver<DroneCommand>,
                     senders: HashMap<u8, Sender<Packet>>,
                     receiver: Receiver<Packet>| {
                        Box::new(ClientAudio::new(
                            client.id,
                            command_send,
                            command_recv,
                            receiver,
                            senders,
                        )) as Box<dyn ClientT>
                    },
                ) as BoxClient,
            ), // TODO Add ClientAudio when implements correct ClientT
            (
                ClientType::Video,
                Box::new(
                    |client: &ParsedClient,
                     command_send: Sender<DroneEvent>,
                     command_recv: Receiver<DroneCommand>,
                     senders: HashMap<u8, Sender<Packet>>,
                     receiver: Receiver<Packet>| {
                        Box::new(ClientVideo::new(
                            client.id,
                            command_send,
                            command_recv,
                            receiver,
                            senders,
                        )) as Box<dyn ClientT>
                    },
                ) as BoxClient,
            ),
        ];

        // Filter factories based on the selected drones
        let filtered_drones = Self::filter_nodes(selected_drones, drone_factories);
        let filtered_clients = Self::filter_nodes(selected_clients, client_factories);

        let initialized_drones = Self::initialize_entities(
            &self.parser.drones,
            &self.channel_map,
            &self.drone_command_map,
            &self.node_event,
            &filtered_drones,
        );

        let initialized_clients = Self::initialize_entities(
            &self.parser.clients,
            &self.channel_map,
            &self.drone_command_map,
            &self.node_event,
            &filtered_clients,
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
        selected_clients: Option<Vec<ClientType>>,
    ) -> Result<(), String> {
        let res: Result<(), String> = match self.state {
            State::Initialized => {
                self.state = State::Running;
                Ok(())
            }
            _ => Err("run_simulation() can only be called when initialized".into()),
        };
        res.as_ref()?;

        let (drones, clients, servers) = self.initialize_network(selected_drones, selected_clients);
        let mut node_handlers: HashMap<NodeId, JoinHandle<()>> = HashMap::new();

        for (i, mut drone) in drones.into_iter().enumerate() {
            node_handlers.insert(
                self.parser.drones[i].id, // Needed because drones do not implement get_id method
                thread::spawn(move || {
                    drone.run();
                }),
            );
        }

        for (i, client) in clients.into_iter().enumerate() {
            let client_id = client.get_id();

            // Determine the path based on the client type
            let init_file_path = if client
                .as_ref()
                .as_any()
                .downcast_ref::<ClientVideo>()
                .is_some()
            {
                "./initialization_files/client_video".to_string()
            } else {
                let client_number = (i % CLIENT_AUDIO_CONFIGURATIONS_NUM) + 1; // Cycle through 1 to 5
                format!("./initialization_files/client_audio/client{client_number}")
            };

            node_handlers.insert(
                client_id,
                thread::spawn(move || {
                    // client.with_all();
                    client.run(&init_file_path);
                }),
            );
        }
        

        for (i, mut server) in servers.into_iter().enumerate() {
            let server_number = (i % SERVER_CONFIGURATIONS_NUM) + 1; // Cycles through 1 to 5
            let init_file_path = format!("./initialization_files/server/server{server_number}");

            node_handlers.insert(
                server.get_id(),
                thread::spawn(move || {
                    server.with_info();
                    server.run(&init_file_path);
                }),
            );
        }

        // // Set up Ctrl+C handler
        // let _command_senders = self.get_controller_senders();
        // ctrlc::set_handler(move || {
        //     println!("Received Ctrl+C, shutting down...");
        //     std::process::exit(0);

        //     // Send crash message to all nodes
        //     // for (id, sender) in &command_senders {
        //     //     sender.send(DroneCommand::Crash).unwrap();
        //     //     println!("Sent crash command to node {}", id);
        //     // }
        // })
        // .expect("Error setting Ctrl+C handler");

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

}
