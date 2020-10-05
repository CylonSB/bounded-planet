use quinn::{ReadExactError, crypto::Session, generic::RecvStream};
use quinn::{generic::SendStream, WriteError};

use tracing::trace;

use crate::networking::packets::*;

impl Packet {
    /// Send this packet to a network stream. Should be read at the other end with `Packet::receive(stream)`
    pub async fn send<T: Session>(&self, stream: &mut SendStream<T>) -> Result<(), SendError> {
        // Encode packet into messagepack format
        let bytes = rmp_serde::to_vec(self).map_err(SendError::EncodeError)?;

        // Prefix with length (4 bytes, network order)
        let len_bytes = (bytes.len() as u32).to_be_bytes();
        stream
            .write_all(&len_bytes)
            .await
            .map_err(SendError::WriteError)?;

        // Write data to socket
        stream
            .write_all(&bytes)
            .await
            .map_err(SendError::WriteError)?;

        trace!("Sent {} bytes", bytes.len());

        Ok(())
    }

    /// Receive a packet from a network stream. Should have been written with `packet.send(stream)`
    pub async fn receive<T: Session>(recv: &mut RecvStream<T>) -> Result<Packet, RecvError> {
        // Read 4 byte network ordered length prefix
        let mut length_prefix_buf = [0u8, 0, 0, 0];
        recv
            .read_exact(&mut length_prefix_buf)
            .await
            .map_err(RecvError::ReadExactError)?;
        let length_prefix = u32::from_be_bytes(length_prefix_buf);

        // Read that many bytes
        let mut data = vec![0; length_prefix as usize];
        recv
            .read_exact(&mut data.as_mut_slice())
            .await
            .map_err(RecvError::ReadExactError)?;

        // Decode it
        let packet: Packet = rmp_serde::from_read_ref(&data).map_err(RecvError::DecodeError)?;

        trace!("Received {} bytes", length_prefix);

        Ok(packet)
    }
}

#[derive(Debug)]
pub enum SendError {

    /// Sending a packet failed due to a serialisation error
    EncodeError(rmp_serde::encode::Error),

    /// Sending a packet failed while writing to the socket
    WriteError(WriteError),
}

#[derive(Debug)]
pub enum RecvError {

    /// Receiving a packet failed due to a deserialisation error
    DecodeError(rmp_serde::decode::Error),

    /// Receiving a packet failed while reading from the socket
    ReadExactError(ReadExactError),
}
