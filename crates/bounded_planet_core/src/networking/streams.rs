use quinn::{ReadExactError, crypto::Session, generic::RecvStream};
use quinn::{generic::SendStream, WriteError};

use tracing::trace;

use crate::networking::packets::*;

/// Wrap a stream, ready to send packets which can be decoded by a `BoundedPlanetRecvStream`
#[derive(Debug)]
pub struct BoundedPlanetSendStream<T: Session> {
    send: SendStream<T>,
}

#[derive(Debug)]
pub enum SendError {

    /// Sending a packet failed due to a serialisation error
    EncodeError(rmp_serde::encode::Error),

    /// Sending a packet failed while writing to the socket
    WriteError(WriteError),
}

impl<TSession: Session> BoundedPlanetSendStream<TSession> {
    pub fn new(send: SendStream<TSession>) -> BoundedPlanetSendStream<TSession> {
        BoundedPlanetSendStream { send }
    }

    /// Send a packet over the network
    pub async fn send_packet(&mut self, packet: &Packet) -> Result<(), SendError> {
        // Encode packet into messagepack format
        let bytes = rmp_serde::to_vec(&packet).map_err(SendError::EncodeError)?;

        // Prefix with length (4 bytes, network order)
        let len_bytes = (bytes.len() as u32).to_be_bytes();
        self.send
            .write_all(&len_bytes)
            .await
            .map_err(SendError::WriteError)?;

        // Write data to socket
        self.send
            .write_all(&bytes)
            .await
            .map_err(SendError::WriteError)?;

        trace!("Sent {} bytes", bytes.len());

        Ok(())
    }
}


/// Wrap a stream, ready to receive packets which were encoded by a `BoundedPlanetRecvStream`
#[derive(Debug)]
pub struct BoundedPlanetRecvStream<T: Session> {
    recv: RecvStream<T>,
}

#[derive(Debug)]
pub enum RecvError {

    /// Receiving a packet failed due to a deserialisation error
    DecodeError(rmp_serde::decode::Error),

    /// Receiving a packet failed while reading from the socket
    ReadExactError(ReadExactError),
}

impl<TSession: Session> BoundedPlanetRecvStream<TSession> {
    pub fn new(recv: RecvStream<TSession>) -> BoundedPlanetRecvStream<TSession> {
        BoundedPlanetRecvStream { recv }
    }

    /// Receive a packet from the network
    pub async fn recv_packet(&mut self) -> Result<Packet, RecvError> {
        // Read 4 byte network ordered length prefix
        let mut length_prefix_buf = [0u8, 0, 0, 0];
        self.recv
            .read_exact(&mut length_prefix_buf)
            .await
            .map_err(RecvError::ReadExactError)?;
        let length_prefix = u32::from_be_bytes(length_prefix_buf);

        // Read that many bytes
        let mut data = Vec::<u8>::with_capacity(length_prefix as usize);
        self.recv
            .read_exact(&mut data.as_mut_slice())
            .await
            .map_err(RecvError::ReadExactError)?;

        // Decode it
        let packet: Packet = rmp_serde::from_read_ref(&data).map_err(RecvError::DecodeError)?;

        trace!("Received {} bytes", length_prefix);

        Ok(packet)
    }
}
