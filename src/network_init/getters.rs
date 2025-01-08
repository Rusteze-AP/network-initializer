use std::collections::HashMap;

use crossbeam::channel::{Receiver, Sender};
use wg_internal::{
    controller::{DroneCommand, DroneEvent},
    network::NodeId,
    packet::Packet,
};

use crate::{
    channel::Channel,
    errors::ConfigError,
    parsed_nodes::{ParsedClient, ParsedDrone, ParsedServer},
    utils::parser::Parser,
};

use super::{NetworkInitializer, State};

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
