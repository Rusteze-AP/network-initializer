use wg_internal::packet::Packet;
use crossbeam::channel::{Receiver, Sender};

// False type to make cargo run happy
// pub type SourceRoutingHeader = [NodeId; 16];

// #[derive(Debug)]
// pub struct Packet {
//     pack_type: PacketType,
//     routing_header: SourceRoutingHeader,
//     session_id: u64,
// }

// #[derive(Debug)]
// pub enum PacketType {
//     MsgFragment(Fragment),
//     Nack(Nack),
//     Ack(Ack),
// }

// #[derive(Debug)]
// pub struct Nack {
//     fragment_index: u64,
//     time_of_fail: std::time::Instant,
//     nack_type: NackType,
// }

// #[derive(Debug)]
// pub enum NackType {
//     ErrorInRouting(NodeId), // contains id of not neighbor
//     Dropped(),
// }

// #[derive(Debug)]
// pub struct Ack {
//     fragment_index: u64,
//     time_received: std::time::Instant,
// }

// #[derive(Debug)]
// pub struct Fragment {
//     fragment_index: u64,
//     total_n_fragments: u64,
//     data: FragmentData,
// }

// #[derive(Debug)]
// pub struct FragmentData {
//     length: u8,
//     data: [u8; 80],
// }

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

// impl FragmentData {
//     pub fn new(length: u8, data: [u8; 80]) -> Self {
//         FragmentData { length, data }
//     }
// }

// impl Fragment {
//     pub fn new(fragment_index: u64, total_n_fragments: u64, data: FragmentData) -> Self {
//         Fragment {
//             fragment_index,
//             total_n_fragments,
//             data,
//         }
//     }
// }

// impl Packet {
//     pub fn new(
//         pack_type: PacketType,
//         routing_header: SourceRoutingHeader,
//         session_id: u64,
//     ) -> Self {
//         Packet {
//             pack_type,
//             routing_header,
//             session_id,
//         }
//     }
// }
