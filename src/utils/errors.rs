use thiserror::Error;
use wg_internal::network::NodeId;

#[derive(Debug, Error, PartialEq)]
pub enum ConfigError {
    #[error("Unable to read config file {0}")]
    FileReadError(String),

    #[error("Unable to parse config file {0}")]
    ParseError(String),

    #[error("Invalid node {0} connection {1}")]
    InvalidNodeConnection(NodeId, NodeId),

    #[error("Unidirectional connection from {0} to {1}")]
    UnidirectionalConnection(NodeId, NodeId),

    #[error("Duplicated node id")]
    DuplicatedNodeId,

    #[error("Empty topology")]
    EmptyTopology,

    #[error("Client {0} with more than 2 connections")]
    ClientWithMoreThanTwoConnections(NodeId),

    #[error("Server {0} with less than 2 connections")]
    ServerWithLessThanTwoConnections(NodeId),
}

#[derive(Debug, Error)]
pub enum ChannelError {
    #[error("Channel not found for node {0}")]
    ChannelNotFound(NodeId),
}
