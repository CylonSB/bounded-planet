use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::{Commands, Entity, EventReader, Events, ResMut};
use quinn::{IncomingUniStreams, crypto::rustls::TlsSession, generic::RecvStream};
use tokio::{
    stream::StreamExt,
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel}
};
use tracing::{error, info, warn};

use super::{
    components::Connection,
    events::{NetworkError, ReceiveEvent, SendEvent},
    id::ConnectionId,
    packets::Packet,
};

/// The stage at which [`SendEvent`]s are sent across the network.
pub const SEND_NET_EVENT_STAGE: &str = bevy::app::stage::LAST;

/// The stage at which [`ReceiveEvent`]s are read from the network.
pub const RECEIVE_NET_EVENT_STAGE: &str = bevy::app::stage::FIRST;

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
    pub stream_senders: HashMap<ConnectionId, UnboundedSender<SendEvent>>,

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
pub fn receive_net_events_system(
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
pub fn send_net_events_system(mut session: ResMut<SessionEventListenerState>, send_events: ResMut<Events<SendEvent>>)
{
    // Publish packets ready to send to appropriate MPSC channels
    for send in session.send_event_reader.iter(&send_events)
    {
        // Try to get the MPSC sender for this connection, early exit if it does not exist
        let connection = send.get_connection();
        let sender = if let Some(sender) = session.stream_senders.get(&connection) {
            sender
        } else {
            error!("Attempted to send to a non-existant connection: {:?}", connection);
            continue;
        };

        // Try to send a message through this MPSC. Sending may fail if the receiving end of the
        // MPSC has been dropped - in that case raise an error.
        if sender.send(send.clone()).is_err() {
            warn!("Failed to publish packet from ECS->MPSC");
            session.event_sender
                .send(ReceiveEvent::NetworkError(send.to_stream_sender_error()))
                .expect("Failed to send error event!");
        }
    }
}

/// Represents a connection that is still in the process of opening
pub struct Connecting {
    id: ConnectionId,
    connecting: quinn::Connecting,
    event_sender: UnboundedSender<ReceiveEvent>,
}

impl Connecting {
    pub fn new(connecting: quinn::Connecting, event_sender: UnboundedSender<ReceiveEvent>) -> Self {
        Connecting {
            id: ConnectionId::new(),
            connecting,
            event_sender
        }
    }

    /// Wait for the connection to finish opening and then spawn the tasks to handle it
    pub async fn run(self) {
        info!("connection incoming: {:?}", self.id);

        // Wait for connection to finish connecting
        let quinn::NewConnection { connection, uni_streams, .. } = match self.connecting.await {
            Ok(connection) => connection,
            Err(e) => {
                self.event_sender.send(ReceiveEvent::NetworkError(
                    NetworkError::ConnectionError(e)
                )).expect("Failed to send network event");
                return;
            }
        };

        // Create a new MPSC which the ECS can use to send packets through this connection
        let (send, recv) = unbounded_channel();

        // Send an intial event indicating that this connection opened
        self.event_sender
            .send(ReceiveEvent::Connected(self.id, send))
            .expect("Failed to send network event");

        // Start running tasks to send/receive to this connection
        tokio::spawn(Connected {
            id: self.id,
            send: self.event_sender,
            connection,
            uni_streams,
            recv
        }.run());
    }    
}

/// Represents a connected quinn connection
struct Connected {
    id: ConnectionId,
    connection: quinn::Connection,
    uni_streams: IncomingUniStreams,
    recv: UnboundedReceiver<SendEvent>,
    send: UnboundedSender<ReceiveEvent>
}

impl Connected {
    /// start running the async tasks required to pump this connection
    pub async fn run(self) {
        // Spawn a task which polls for new incoming streams
        tokio::spawn(Self::poll_incoming_streams(self.uni_streams, self.send.clone(), self.id));

        // Spawn a task which opens new outgoing streams and sends packets to them
        tokio::spawn(Self::send_to_streams(self.connection, self.recv, self.send));
    }

    /// keep watch for new incoming streams and spawn async tasks to send/receive to the stream
    async fn poll_incoming_streams(mut uni_streams: IncomingUniStreams, event_sender: UnboundedSender<ReceiveEvent>, id: ConnectionId) {
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
                    tokio::spawn(Self::read_from_stream(id, recv, event_sender.clone()));
                }
            };
        }

        // Send a final event indicating that this connection closed
        event_sender
            .send(ReceiveEvent::Disconnected(id))
            .expect("Failed to send network event");
    }

    /// Pump the MPSCs for new packets that need sending
    async fn send_to_streams(
        mut conn: quinn::Connection,
        mut recv: UnboundedReceiver<SendEvent>,
        event_sender: UnboundedSender<ReceiveEvent>
    ) {
        // Create a list of open streams. When a request comes in to send over a non-existant stream open it and store it here.
        // Streams are never closed. This is fine since there are a fixed number of streams (as defined in the StreamType enum).
        let mut stream_lookup = Vec::new();
    
        // Keep pulling events from the stream until "None" is received (indicating that all senders have been dropped).
        while let Some(evt) = recv.recv().await
        {
            // Send the packet and break out of the loop if sending errorred
            if let Err(err) = evt.send(&mut stream_lookup, &mut conn).await {
                event_sender.send(ReceiveEvent::NetworkError(err)).expect("Failed to send error event!");
                break;
            }
        }
    }

    /// Handle all the work of reading from a specific stream
    async fn read_from_stream(
        connection_id: ConnectionId,
        mut stream_recv: RecvStream<TlsSession>,
        event_sender: UnboundedSender<ReceiveEvent>,
    ) {
        info!("Stream incoming: conn:{:?}", connection_id);

        // Pull packets from this stream and publish them to the ECS through the event_sender
        loop {
            let pkt = Packet::receive(&mut stream_recv).await;
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
}
