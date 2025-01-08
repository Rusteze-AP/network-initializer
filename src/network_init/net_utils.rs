use std::collections::HashMap;

use crossbeam::channel::{Receiver, Sender};
use wg_internal::{
    controller::{DroneCommand, DroneEvent},
    drone::Drone,
    packet::Packet,
};

use crate::parsed_nodes::ParsedDrone;

pub(crate) type BoxDrone = Box<
    dyn Fn(
        &ParsedDrone,
        Sender<DroneEvent>,
        Receiver<DroneCommand>,
        HashMap<u8, Sender<Packet>>,
        Receiver<Packet>,
    ) -> Box<dyn Drone>,
>;

// Macro that creates a vector of `BoxDrone`
#[macro_export]
macro_rules! create_drone_factories {
    ($($drone_type:ident),*) => {
        vec![
            $(
                Box::new(|parsed_drone: &ParsedDrone, command_send: Sender<DroneEvent>, command_recv: Receiver<DroneCommand>, senders: HashMap<u8, Sender<Packet>>, receiver: Receiver<Packet>| {
                    Box::new($drone_type::new(
                        parsed_drone.id,
                        command_send,
                        command_recv,
                        receiver,
                        senders,
                        parsed_drone.pdr,
                    )) as Box<dyn Drone>
                }) as BoxDrone
            ),*
        ]
    };
}
