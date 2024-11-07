use crate::NodeId;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Unable to read config file {0}")]
    FileReadError(String),

    #[error("Unable to parse config file {0}")]
    ParseError(String),

    #[error("Invalid drone {0} connection {1}")]
    InvalidDroneConnection(NodeId, NodeId),

    #[error("Invalid client {0} connection {1}")]
    InvalidClientConnection(NodeId, NodeId),

    #[error("Invalid server {0} connection {1}")]
    InvalidServerConnection(NodeId, NodeId),
}
