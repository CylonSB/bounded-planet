use std::sync::Arc;

use crate::networking::{id::ConnectionId, packets::Packet};
use tokio::sync::mpsc::UnboundedSender;

use super::{packets::StreamType, streams::{RecvError, SendError}};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum NetworkError {
    /// An error occured in attempting to receive a packet from the given stream
    #[error("Unable to receive packet from connection {connection:?}. Underlying error: {err:?}")]
    ReceiveError {
        connection: ConnectionId,
        err: RecvError,
    },

    /// An error occured during sending
    #[error("Unable to send packet on stream {stream:?} of connection {connection:?}. Underlying error: {err:?}")]
    SendError {
        connection: ConnectionId,
        stream: StreamType,
        err: SendError,
    },

    /// An error occured attempting to send a packet through the MPSC to the async packet sender loop
    #[error("MPSC to packet sender closed for stream {stream:?} of connection {connection:?}. Packet that wasn't sent: {failed_packet:?}")]
    StreamSenderError {
        connection: ConnectionId,
        stream: StreamType,
        failed_packet: Arc<Packet>,
    },

    /// An error occurred in quinn while attempting to connect
    #[error("Quinn connection error: {0:?}")]
    ConnectionError(#[from] quinn::ConnectionError)
}

/// An event generated by the network
#[derive(Debug)]
pub enum ReceiveEvent {

    /// A new connection has opened
    Connected(ConnectionId, UnboundedSender<(StreamType, Arc<Packet>)>),

    /// A packet has arrived in a stream
    ReceivedPacket {
        connection: ConnectionId,
        data: Arc<Packet>,
    },

    /// A connection has closed
    Disconnected(ConnectionId),

    /// The network socket has closed
    SocketClosed,

    /// An error occurred from the socket
    NetworkError(NetworkError),
}

/// An event to send to the network
#[derive(Debug)]
pub enum SendEvent {

    /// Send a packet through a specific stream on a specific connection
    SendPacket {
        connection: ConnectionId,
        stream: StreamType,
        data: Arc<Packet>,
    },
}
