use std::{net::SocketAddr, sync::Arc};

use bevy::prelude::{AppBuilder, Plugin};
use quinn::{crypto::rustls::TlsSession, generic::Incoming};

use bevy::prelude::IntoQuerySystem;
use futures::StreamExt;

use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tracing::info;

use crate::networking::{
    events::{
        ReceiveEvent,
        SendEvent
    },
    systems::*
};

/// Add this plugin to start a server which sends and receives packets to a large number of network connections
#[derive(Debug)]
pub struct Network {
    pub private_key: quinn::PrivateKey,
    pub certificate: quinn::CertificateChain,
    pub addr: SocketAddr,
}

impl Plugin for Network {
    fn build(&self, app: &mut AppBuilder) {
        // Create mpsc endpoints for received network events and store them in a resources
        let (send, recv) = unbounded_channel();
        app.add_resource(SessionEventListenerState {
            event_sender: send.clone(),
            event_receiver: recv,
            stream_senders: Default::default(),
            send_event_reader: Default::default(),
        });

        app.init_resource::<NetworkConnections>();

        app.add_event::<ReceiveEvent>();
        app.add_event::<SendEvent>();

        // Create listen socket
        let listening = create_endpoint(
            self.addr,
            self.private_key.clone(),
            self.certificate.clone(),
        )
        .expect("Failed to create socket");

        // Spawn a task that polls the socket for events and sends them into an mspc
        tokio::spawn(poll_new_connections(listening, send));

        // Add a system that consumes all network events from an MPSC and
        // publishes them as ECS events
        app.add_system_to_stage(RECEIVE_NET_EVENT_STAGE, receive_net_events.system());

        // Add a system that consumes ECS events and forwards them to MPSCs
        // which will eventually be sent over the network
        app.add_system_to_stage(SEND_NET_EVENT_STAGE, send_net_events.system());
    }
}

/// Poll for new incoming connection requests
async fn poll_new_connections(
    mut incoming: Incoming<TlsSession>,
    event_sender: UnboundedSender<ReceiveEvent>,
) {
    info!("Polling for incoming connections");

    // Keep polling for new incoming connections being opened
    while let Some(conn) = incoming.next().await {
        tokio::spawn(handle_connection(conn, event_sender.clone()));
    }

    // Once the socket has closed notify the ECS about it. If sending this
    // fails (because the ECS has stopped listening) just silently give up.
    info!("Socket closed");
    let _ = event_sender.send(ReceiveEvent::SocketClosed);
}

/// Create a network endpoint
fn create_endpoint(
    listen: SocketAddr,
    private_key: quinn::PrivateKey,
    certificate: quinn::CertificateChain,
) -> Result<Incoming<TlsSession>, Box<dyn std::error::Error>> {
    // Configure endpoint
    let mut transport_config = quinn::TransportConfig::default();
    transport_config.stream_window_uni(128);
    let mut server_config = quinn::ServerConfig::default();
    server_config.transport = Arc::new(transport_config);
    let mut server_config = quinn::ServerConfigBuilder::new(server_config);
    server_config.protocols(&[b"hq-29"]);

    // Configure encryption
    server_config.certificate(
        certificate,
        private_key,
    )?;

    // Begin listening for connections, drop the endpoint because we don't need
    // to establish any outgoing connections
    let mut endpoint = quinn::Endpoint::builder();
    endpoint.listen(server_config.build());
    let (_, incoming) = endpoint.bind(&listen)?;

    Ok(incoming)
}
