use uuid::Uuid;

/// Uniquely identifies a single network connection
#[derive(Debug, Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ConnectionId(Uuid);

impl ConnectionId {
    pub fn new() -> ConnectionId {
        ConnectionId(Uuid::new_v4())
    }
}

impl Default for ConnectionId {
    fn default() -> Self {
        Self::new()
    }
}
