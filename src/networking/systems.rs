use std::{collections::HashMap};

use bevy::prelude::{Commands, Entity, EventReader, Events, ResMut};
use quinn::{crypto::rustls::TlsSession, generic::{RecvStream, SendStream}};
use tokio::{stream::StreamExt, sync::mpsc::UnboundedReceiver, sync::mpsc::UnboundedSender, sync::mpsc::unbounded_channel};
use tracing::{error, info};

use super::{components::Connection, events::ReceiveEvent, events::SendEvent, id::ConnectionId, id::StreamId, packets::Packet, streams::BoundedPlanetRecvStream, streams::BoundedPlanetSendStream};

/// Internal state of the network session system
pub struct SessionEventListenerState {
    /// Map from ConnectionID => StreamId => StreamSender
    pub stream_senders: HashMap<ConnectionId, HashMap<StreamId, UnboundedSender<Packet>>>,

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

// Consume events from ECS and publish them to the correct MPSC to send it over the network
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
            ReceiveEvent::Connected(id) => {

                // Create an entity representing this connection
                commands.spawn((
                    Connection { id },
                ));
                entities.connections.insert(id.clone(), commands.current_entity().expect("`spawn` did not create an entity"));

                // Create a hashmap to hold stream MPSCs for this connection
                session
                    .stream_senders
                    .insert(id.clone(), Default::default());
            }

            ReceiveEvent::Disconnected(id) => {

                // Delete the entity representing this connection
                if let Some(e) = entities.connections.remove(&id) {
                    commands.despawn(e);
                }

                // drop all stream senders
                session.stream_senders.remove(&id);
            }

            // Store the MPSC to send to this stream in the session state
            ReceiveEvent::OpenedStream {
                connection_id,
                stream_id,
                ref sender,
            } => {
                session
                    .stream_senders
                    .entry(connection_id.clone())
                    .or_default()
                    .insert(stream_id.clone(), sender.clone());
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

// Take ECS events and forward them to MPSCs to be sent over the network
pub fn send_net_events(mut session: ResMut<SessionEventListenerState>, send_events: ResMut<Events<SendEvent>>) {
    // Publish packets ready to send to appropriate MPSC channels
    for send in session.send_event_reader.iter(&send_events) {
        match send {
            SendEvent::SendPacket { connection_id, stream_id, data } => {
                let sender = session
                    .stream_senders
                    .entry(connection_id.clone())
                    .or_default()
                    .get(&stream_id);

                if let Some(sender) = sender {
                    if let Err(e) = sender.send(data.to_owned()) {
                        error!("Send Error: {:?}", e);
                    }
                } else {
                    error!(
                        "Attempted to send to a non-existant stream: {:?}:{:?}",
                        connection_id, stream_id
                    );
                }
            }
        }
    }
}

// Handle all the work of a single connection (waiting for new streams to open)
pub async fn handle_connection(
    conn: quinn::Connecting,
    event_sender: UnboundedSender<ReceiveEvent>,
) -> Result<(), ()> {
    // Generate a unique ID for this connection
    let guid = ConnectionId::new();
    info!("connection incoming: {:?}", guid);

    // Wait for connection to finish connecting
    let quinn::NewConnection { mut bi_streams, .. } = conn
        .await
        .map_err(|e| error!("Connection Error: {:?}", e))?;

    // Send an intial event indicating that this connection opened
    event_sender
        .send(ReceiveEvent::Connected(guid))
        .expect("Failed to send network event");

    // Keep getting events from the connection until it closes
    while let Some(stream) = bi_streams.next().await {
        match stream {
            Err(quinn::ConnectionError::ApplicationClosed { .. }) => break,
            Err(e) => error!("Connection error: {:?}", e),
            Ok((send, recv)) => {
                tokio::spawn(handle_stream(guid, send, recv, event_sender.clone()));
            }
        };
    }

    // Send a final event indicating that this connection closed
    event_sender
        .send(ReceiveEvent::Disconnected(guid))
        .expect("Failed to send network event");

    Ok(())
}

// Handle all the work of a specific stream
async fn handle_stream(
    connection_id: ConnectionId,
    stream_send: SendStream<TlsSession>,
    stream_recv: RecvStream<TlsSession>,
    event_sender: UnboundedSender<ReceiveEvent>,
) {
    // Generate a unique ID for this connection
    let stream_id = StreamId::new();
    info!(
        "Stream incoming: conn:{:?} stream:{:?}",
        connection_id, stream_id
    );

    // Create an mpsc for sending messages through this stream
    let (send, recv) = unbounded_channel();

    // Send an intial event indicating that this stream opened, along with the mpsc to send to this stream
    event_sender
        .send(ReceiveEvent::OpenedStream {
            connection_id,
            stream_id,
            sender: send,
        })
        .expect("Failed to send network event");

    // Spawn a task which pulls packets from this stream and publishes them to the event_sender
    tokio::spawn(recv_from_stream(
        connection_id,
        stream_id,
        stream_recv,
        event_sender,
    ));

    // Spawn a task which pulls messages from the ECS and sends them to this stream
    tokio::spawn(send_to_stream(stream_send, recv));
}

// Pull packets from socket and push into an mpsc
async fn recv_from_stream(
    connection_id: ConnectionId,
    stream_id: StreamId,
    recv: RecvStream<TlsSession>,
    event_sender: UnboundedSender<ReceiveEvent>,
) {
    let mut recv = BoundedPlanetRecvStream::new(recv);

    loop {
        let pkt = recv.recv_packet().await;
        let se = match pkt {
            Ok(pkt) => event_sender.send(ReceiveEvent::ReceivedPacket {
                connection_id,
                stream_id,
                data: pkt,
            }),
            Err(err) => event_sender.send(ReceiveEvent::ReceiveError {
                connection_id,
                stream_id,
                err,
            }),
        };
        se.expect("Failed to send event");
    }
}

/// Pull packets from an mpsc and send them to the given stream
async fn send_to_stream(send: SendStream<TlsSession>, mut recv: UnboundedReceiver<Packet>) {
    let mut send = BoundedPlanetSendStream::new(send);
    while let Some(pkt) = recv.recv().await {
        if let Err(e) = send.send_packet(&pkt).await {
            // todo expose this error in a more useful way
            error!("Send Error: {:?}", e);
        }
    }
}
