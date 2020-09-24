use std::net::SocketAddr;

use bevy::prelude::{AppBuilder, Plugin, IntoQuerySystem};
use quinn::{crypto::rustls::TlsSession, generic::Connecting};
use tokio::sync::mpsc::unbounded_channel;
use url::Url;

use crate::networking::{events::{ReceiveEvent, SendEvent}, systems::{NetworkConnections, SessionEventListenerState, handle_connection, receive_net_events, send_net_events}};

pub struct Network {
    pub addr: SocketAddr,
    pub url: Url,
    pub cert: quinn::CertificateChain
}

impl Plugin for Network {
    fn build(&self, app: &mut AppBuilder) {
        // Create mpsc endpoints for received network events and store them in a resources
        let (send, recv) = unbounded_channel();
        app.add_resource(SessionEventListenerState {
            event_receiver: recv,
            stream_senders: Default::default(),
            send_event_reader: Default::default(),
        });

        app.init_resource::<NetworkConnections>();
        app.add_resource::<NetworkConnections>(Default::default());

        app.add_event::<ReceiveEvent>();
        app.add_event::<SendEvent>();

        // Start a task that waits for the connection to finish opening
        tokio::spawn(handle_connection(create_endpoint(&self.addr, &self.url, &self.cert).expect("Failed to create an endpoint"), send.clone()));

        // Add a system that consumes all network events from an MPSC and publishes them as ECS events
        app.add_system(receive_net_events.system());

        // Add a system that consumes ECS events and forwards them to MPSCs which will eventually be sent over the network
        app.add_system(send_net_events.system());
    }
}

fn create_endpoint(addr: &SocketAddr, url: &Url, server_cert: &quinn::CertificateChain) -> Result<Connecting<TlsSession>, Box<dyn std::error::Error>> {
    let mut endpoint = quinn::Endpoint::builder();
    let mut client_config = quinn::ClientConfigBuilder::default();
    client_config.protocols(&[b"hq-29"]);

    //todo:add a server authority here
    //client_config.add_certificate_authority(server_cert);

    endpoint.default_client_config(client_config.build());

    let (endpoint, _) = endpoint.bind(&"[::]:0".parse().unwrap())?;
    let connecting = endpoint.connect(addr, &url.host_str().unwrap())?;

    return Ok(connecting);
}