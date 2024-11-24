use crossbeam::channel::{Receiver, Sender, TryRecvError};
use std::{collections::HashMap, thread, time::Duration};
use wg_internal::network::NodeId;
use wg_internal::packet::Packet;

#[derive(Debug)]
pub struct Client {
    pub id: NodeId,
    pub receiver: Receiver<Packet>,
    pub senders: HashMap<NodeId, Sender<Packet>>,
}

pub trait ClientTrait {
    fn new(
        id: NodeId,
        receiver: Receiver<Packet>,
        senders: HashMap<NodeId, Sender<Packet>>,
    ) -> Self;
    fn run(&self);
}

pub struct Server {
    pub id: NodeId,
    pub receiver: Receiver<Packet>,
    pub senders: HashMap<NodeId, Sender<Packet>>,
}

pub trait ServerTrait {
    fn new(
        id: NodeId,
        receiver: Receiver<Packet>,
        senders: HashMap<NodeId, Sender<Packet>>,
    ) -> Self;
    fn run(&self);
}

impl ClientTrait for Client {
    fn new(
        id: NodeId,
        receiver: Receiver<Packet>,
        senders: HashMap<NodeId, Sender<Packet>>,
    ) -> Self {
        Client {
            id,
            receiver,
            senders,
        }
    }

    fn run(&self) {
        loop {
            thread::sleep(Duration::from_secs(1));

            // Check if there's a message from the drone
            match self.receiver.try_recv() {
                Ok(packet) => {
                    println!("Client {} received a message: {:?}", self.id, packet);
                }
                Err(TryRecvError::Empty) => {
                    println!("No messages for client {}", self.id);
                }
                Err(err) => {
                    eprintln!("Error receiving message for client {}: {:?}", self.id, err);
                }
            }

            // if self.id == 5 {
            //     let frag_data = FragmentData::new(1, [2; 80]);
            //     let fragment = Fragment::new(0, 1, frag_data);
            //     let packet = Packet::new(PacketType::MsgFragment(fragment), [2; 16], 1);
            //     if let Some(sender) = self.senders.get(&2) {
            //         sender.send(packet).unwrap();
            //         println!("Client {} sent packet to node 2", self.id);
            //     } else {
            //         println!("Client {} could not send packet to node 2", self.id);
            //     }
            // }
        }
    }
}

impl ServerTrait for Server {
    fn new(
        id: NodeId,
        receiver: Receiver<Packet>,
        senders: HashMap<NodeId, Sender<Packet>>,
    ) -> Self {
        Server {
            id,
            receiver,
            senders,
        }
    }

    fn run(&self) {
        loop {
            thread::sleep(Duration::from_secs(1));
        }
    }
}
