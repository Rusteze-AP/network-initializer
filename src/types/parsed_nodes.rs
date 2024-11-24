use serde::Deserialize;
use wg_internal::network::NodeId;

#[derive(Debug, Deserialize)]
pub struct ParsedDrone {
    pub id: NodeId,
    pub connected_drone_ids: Vec<NodeId>,
    pub pdr: f32,
}

#[derive(Debug, Deserialize)]
pub struct ParsedClient {
    pub id: NodeId,
    pub connected_drone_ids: Vec<NodeId>,
}

#[derive(Debug, Deserialize)]
pub struct ParsedServer {
    pub id: NodeId,
    pub connected_drone_ids: Vec<NodeId>,
}

pub trait Initializable {
    fn id(&self) -> &NodeId;
    fn connected_drone_ids(&self) -> &Vec<NodeId>;
}

impl Initializable for ParsedDrone {
    fn id(&self) -> &NodeId {
        &self.id
    }

    fn connected_drone_ids(&self) -> &Vec<NodeId> {
        &self.connected_drone_ids
    }
}

impl Initializable for ParsedClient {
    fn id(&self) -> &NodeId {
        &self.id
    }

    fn connected_drone_ids(&self) -> &Vec<NodeId> {
        &self.connected_drone_ids
    }
}

impl Initializable for ParsedServer {
    fn id(&self) -> &NodeId {
        &self.id
    }

    fn connected_drone_ids(&self) -> &Vec<NodeId> {
        &self.connected_drone_ids
    }
}
