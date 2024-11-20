use serde::Deserialize;
pub type NodeId = u64;

#[derive(Debug, Deserialize)]
pub struct Drone {
    pub id: NodeId,
    pub connected_drone_ids: Vec<NodeId>,
    pub pdr: f64,
}

#[derive(Debug, Deserialize)]
pub struct Client {
    pub id: NodeId,
    pub connected_drone_ids: Vec<NodeId>,
}

#[derive(Debug, Deserialize)]
pub struct Server {
    pub id: NodeId,
    pub connected_drone_ids: Vec<NodeId>,
}

pub trait Initializable {
    fn id(&self) -> &NodeId;
    fn connected_drone_ids(&self) -> &Vec<NodeId>;
}

impl Initializable for Drone {
    fn id(&self) -> &NodeId {
        &self.id
    }

    fn connected_drone_ids(&self) -> &Vec<NodeId> {
        &self.connected_drone_ids
    }
}

impl Initializable for Client {
    fn id(&self) -> &NodeId {
        &self.id
    }

    fn connected_drone_ids(&self) -> &Vec<NodeId> {
        &self.connected_drone_ids
    }
}

impl Initializable for Server {
    fn id(&self) -> &NodeId {
        &self.id
    }

    fn connected_drone_ids(&self) -> &Vec<NodeId> {
        &self.connected_drone_ids
    }
}
