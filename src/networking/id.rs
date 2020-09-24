use uuid::Uuid;

/// Uniquely identifies a single network connection
#[derive(Debug, Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ConnectionId(Uuid);

impl ConnectionId {
    pub fn new() -> ConnectionId {
        ConnectionId(Uuid::new_v4())
    }
}

/// Uniquely identifies a single bidirectional stream of data within a single network connections
#[derive(Debug, Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StreamId(Uuid);

impl StreamId {
    pub fn new() -> StreamId {
        StreamId(Uuid::new_v4())
    }
}
