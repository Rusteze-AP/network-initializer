use serde::Deserialize;
use wg_internal::network::NodeId;

#[derive(PartialEq)]
pub enum NodeType {
    Drone,
    Client,
    Server,
}

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

pub trait Node {
    fn id(&self) -> NodeId;
    fn connected_drone_ids(&self) -> &Vec<NodeId>;
    fn node_type(&self) -> NodeType;
}

impl Node for ParsedDrone {
    fn id(&self) -> NodeId {
        self.id
    }

    fn connected_drone_ids(&self) -> &Vec<NodeId> {
        &self.connected_drone_ids
    }

    fn node_type(&self) -> NodeType {
        NodeType::Drone
    }
}

impl Node for ParsedClient {
    fn id(&self) -> NodeId {
        self.id
    }

    fn connected_drone_ids(&self) -> &Vec<NodeId> {
        &self.connected_drone_ids
    }

    fn node_type(&self) -> NodeType {
        NodeType::Client
    }
}

impl Node for ParsedServer {
    fn id(&self) -> NodeId {
        self.id
    }

    fn connected_drone_ids(&self) -> &Vec<NodeId> {
        &self.connected_drone_ids
    }

    fn node_type(&self) -> NodeType {
        NodeType::Server
    }
}
