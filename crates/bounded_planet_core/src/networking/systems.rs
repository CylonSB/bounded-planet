use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::{Commands, Entity, EventReader, Events, ResMut};
use quinn::{ConnectionError, IncomingUniStreams, crypto::rustls::TlsSession, generic::RecvStream};
use tokio::{
    stream::StreamExt,
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel}
};
use tracing::{error, info, warn};

use super::{
    components::Connection,
    events::{NetworkError, ReceiveEvent, SendEvent},
    id::ConnectionId,
    packets::{Packet, StreamType},
    streams::{BoundedPlanetRecvStream, BoundedPlanetSendStream}
};

#[derive(Default)]
pub struct NetEventLoggerState {
    pub event_reader: EventReader<ReceiveEvent>,
}

pub fn log_net_events(mut state: ResMut<NetEventLoggerState>, receiver: ResMut<Events<ReceiveEvent>>) {
    for evt in state.event_reader.iter(&receiver) {
        match evt {
            ReceiveEvent::Connected(cid, _) => info!("New Connection: {:?}", cid),
            ReceiveEvent::Disconnected(cid) => info!("Disconnected: {:?}", cid),
            ReceiveEvent::SocketClosed => warn!("Socket Closed"),
            ReceiveEvent::NetworkError(err) => error!("Network Error: {:?}", err),

            _ => {}
        }
    }
}

/// Internal state of the network session system
pub struct SessionEventListenerState {
    /// Map from ConnectionID => PacketSender MPSC
    pub stream_senders: HashMap<ConnectionId, UnboundedSender<(StreamType, Arc<Packet>)>>,

    /// MPSC sender for the accompanying event_receiver
    pub event_sender: UnboundedSender<ReceiveEvent>,

    /// MPSC which receives all events from the network
    pub event_receiver: UnboundedReceiver<ReceiveEvent>,

    /// Event reader used for pulling send events and publishing them to the network
    pub send_event_reader: EventReader<SendEvent>,
}

/// ECS resources containing a map of active network connections
#[derive(Default)]
pub struct NetworkConnections {
    pub connections: HashMap<ConnectionId, Entity>
}

/// Consume events from the network (sent through MPSCs) and publish them to the ECS
pub fn receive_net_events(
    mut commands: Commands,
    mut session: ResMut<SessionEventListenerState>,
    mut entities: ResMut<NetworkConnections>,
    mut net_events: ResMut<Events<ReceiveEvent>>
) {
    // Break up `session` in a way that Rust is happy with
    let session: &mut SessionEventListenerState = &mut session;
    let SessionEventListenerState { event_receiver, .. } = session;

    // Pull network events from MPSC and publish them
    while let Ok(event) = event_receiver.try_recv() {
        match event {
            
            // A new connection has opened, allocate an entry in the hashmap
            ReceiveEvent::Connected(id, ref packet_sender) => {

                // Create an entity representing this connection
                commands.spawn((
                    Connection { id },
                ));
                entities.connections.insert(id, commands.current_entity().expect("`spawn` did not create an entity"));

                // Store the MPSC to send to this stream in the hashmap
                session.stream_senders.insert(id, packet_sender.clone());
            }

            ReceiveEvent::Disconnected(id) => {

                // Delete the entity representing this connection
                if let Some(e) = entities.connections.remove(&id) {
                    commands.despawn(e);
                } else {
                    warn!("Failed to delete connection Entity for ConnectionId:{:?}", id);
                }

                // drop all stream senders
                session.stream_senders.remove(&id);
            }

            // When the socket closes throw away all session state
            ReceiveEvent::SocketClosed => {

                // Delete all connection entities
                for (_, e) in entities.connections.drain() {
                    commands.despawn(e);
                }

                // drop all stream senders
                session.stream_senders.clear();
            }

            _ => {}
        }

        // Publish event for other systems to consume
        net_events.send(event);
    }
}

/// Take ECS events and forward them to MPSCs to be sent over the network
pub fn send_net_events(mut session: ResMut<SessionEventListenerState>, send_events: ResMut<Events<SendEvent>>) {
    // Publish packets ready to send to appropriate MPSC channels
    for send in session.send_event_reader.iter(&send_events) {
        match send {
            SendEvent::SendPacket { connection, stream, data } => {
                if let Some(sender) = session.stream_senders.get(&connection) {
                    if let Err(e) = sender.send((*stream, Arc::clone(data))) {
                        session.event_sender.send(ReceiveEvent::NetworkError(
                            NetworkError::StreamSenderError {
                                connection: *connection,
                                stream: *stream,
                                failed_packet: e.0.1
                            }
                        )).expect("Failed to send error event!");

                        warn!("Failed to publish packet from ECS->MPSC");
                    }
                } else {
                    error!("Attempted to send to a non-existant connection: {:?}", connection);
                }
            }
        }
    }
}

/// Handle all the work of a single connection (waiting for new streams to open)
pub async fn handle_connection(
    conn: quinn::Connecting,
    event_sender: UnboundedSender<ReceiveEvent>,
) {
    // Generate a unique ID for this connection
    let cid = ConnectionId::new();
    info!("connection incoming: {:?}", cid);

    // Wait for connection to finish connecting
    let quinn::NewConnection { connection, uni_streams, .. } = match conn.await {
        Ok(connection) => connection,
        Err(e) => {
            event_sender.send(ReceiveEvent::NetworkError(
                NetworkError::ConnectionError(e)
            )).expect("Failed to send network event");
            return;
        }
    };

    // Create a new MPSC which the ECS can use to send packets through this connection
    let (send, recv) = unbounded_channel();

    // Send an intial event indicating that this connection opened
    event_sender
        .send(ReceiveEvent::Connected(cid, send))
        .expect("Failed to send network event");

    // Spawn a task which polls for new incoming streams
    tokio::spawn(handle_incoming_streams(uni_streams, event_sender.clone(), cid));

    // Spawn a task which opens new outgoing streams and sends packets to them
    tokio::spawn(send_to_streams(cid, connection, recv, event_sender));
}

async fn handle_incoming_streams(
    mut uni_streams: IncomingUniStreams,
    event_sender: UnboundedSender<ReceiveEvent>,
    id: ConnectionId
) {
    // Keep getting events from the connection until it closes
    while let Some(stream) = uni_streams.next().await {
        match stream {
            Err(quinn::ConnectionError::ApplicationClosed { .. }) => break,
            Err(e) => {
                event_sender.send(ReceiveEvent::NetworkError(
                    NetworkError::ConnectionError(e)
                )).expect("Failed to send network event");
                break;
            },
            Ok(recv) => {
                tokio::spawn(read_from_stream(id, recv, event_sender.clone()));
            }
        };
    }

    // Send a final event indicating that this connection closed
    event_sender
        .send(ReceiveEvent::Disconnected(id))
        .expect("Failed to send network event");
}

/// Handle all the work of a specific stream
async fn read_from_stream(
    connection_id: ConnectionId,
    stream_recv: RecvStream<TlsSession>,
    event_sender: UnboundedSender<ReceiveEvent>,
) {
    info!("Stream incoming: conn:{:?}", connection_id);

    // Pull packets from this stream and publish them to the ECS through the event_sender
    let mut recv = BoundedPlanetRecvStream::new(stream_recv);
    loop {
        let pkt = recv.recv_packet().await;
        let se = match pkt {
            Ok(pkt) => event_sender.send(ReceiveEvent::ReceivedPacket {
                connection: connection_id,
                data: Arc::new(pkt),
            }),
            Err(err) => {
                event_sender.send(ReceiveEvent::NetworkError(
                    NetworkError::ReceiveError {
                        connection: connection_id,
                        err,
                    }
                )).expect("Failed to send error event!");
                break;
            },
        };
        se.expect("Failed to send event");
    }
}

async fn send_to_streams(
    connection_id: ConnectionId,
    mut conn: quinn::Connection,
    mut recv: UnboundedReceiver<(StreamType, Arc<Packet>)>,
    event_sender: UnboundedSender<ReceiveEvent>
) {

    // Create a list of open streams. When a request comes in to send over a non-existant stream open it and store it here.
    // Streams are never closed. This is fine since there are a fixed number fo streams (as defined in the StreamType enum).
    let mut stream_lookup = Vec::new();

    loop {
        match recv.recv().await {

            // Once the receiver receives `None` that indicates all senders have been dropped. This loop can safely end because
            // it's impossible for any more messages to arrive at this MPSC receiver.
            None => { break; }

            // Send a packet through a stream
            Some((stream_type, pkt)) => {
                // Find (or create) the sender for this stream
                let sender = match get_stream_sender(&stream_type, &mut stream_lookup, &mut conn).await {
                    Ok(sender) => sender,
                    Err(err) => {
                        event_sender.send(ReceiveEvent::NetworkError(
                             NetworkError::ConnectionError(err)
                        )).expect("Failed to send error event!");
                        break;
                    },
                };

                // Send the packet through the stream
                if let Err(err) = sender.send_packet(&pkt).await {
                    event_sender.send(ReceiveEvent::NetworkError(
                        NetworkError::SendError {
                            connection: connection_id,
                            stream: stream_type,
                            err,
                        }
                    )).expect("Failed to send error event!");
                }
            }
        }
    }
}

async fn get_stream_sender<'a>(
    stream_type: &StreamType,
    stream_lookup: &'a mut Vec<BoundedPlanetSendStream<TlsSession>>,
    conn: &mut quinn::Connection
) -> Result<&'a mut BoundedPlanetSendStream<TlsSession>, ConnectionError>
{
    // Calculate the index of this sender simply as the enum variant index
    let idx = *stream_type as usize;

    // Keep opening streams until the list of senders is large enough
    if idx >= stream_lookup.len() {
        stream_lookup.reserve(idx - stream_lookup.len() + 1);
    }
    while idx >= stream_lookup.len() {
        stream_lookup.push(BoundedPlanetSendStream::new(conn.open_uni().await?));
    }

    // The list is now large enough that the sender definitely exists
    Ok(stream_lookup.get_mut(idx).expect("List was just grown to this size"))
}
