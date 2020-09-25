use super::id::ConnectionId;

/// This component represents a network connection with the given ID
#[derive(Debug, Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Connection {
    pub id: ConnectionId
}