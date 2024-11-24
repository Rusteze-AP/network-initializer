use crossbeam::channel::{Receiver, Sender};
use wg_internal::packet::Packet;

#[derive(Debug)]
pub struct Channel {
    pub sender: Sender<Packet>,
    pub receiver: Receiver<Packet>,
}

impl Channel {
    pub fn new(sender: Sender<Packet>, receiver: Receiver<Packet>) -> Self {
        Channel { sender, receiver }
    }
}
