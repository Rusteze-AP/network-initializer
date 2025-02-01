use std::collections::HashMap;

use crossbeam::channel::{Receiver, Sender};
use packet_forge::ClientT;
use wg_internal::{
    controller::{DroneCommand, DroneEvent},
    drone::Drone,
    packet::Packet,
};

use crate::parsed_nodes::{ParsedClient, ParsedDrone};

pub(crate) type BoxDrone = Box<
    dyn Fn(
        &ParsedDrone,
        Sender<DroneEvent>,
        Receiver<DroneCommand>,
        HashMap<u8, Sender<Packet>>,
        Receiver<Packet>,
    ) -> Box<dyn Drone>,
>;

pub(crate) type BoxClient = Box<
    dyn Fn(
        &ParsedClient,
        Sender<DroneEvent>,
        Receiver<DroneCommand>,
        HashMap<u8, Sender<Packet>>,
        Receiver<Packet>,
    ) -> Box<dyn ClientT>,
>;

// Macro that creates a vector of `(DroneType, BoxDrone)`
#[macro_export]
macro_rules! create_drone_factories {
    ($($drone_type:ident),*) => {
        vec![
            $(
                (
                    DroneType::$drone_type,
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
                )
            ),*
        ]
    };
}
