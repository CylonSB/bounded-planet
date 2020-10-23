use std::{net::SocketAddr, sync::Arc};

use thiserror::Error;
use bevy::prelude::{AppBuilder, Plugin, IntoQuerySystem};
use quinn::{
    ClientConfigBuilder,
    crypto::rustls::TlsSession,
};
use tokio::sync::mpsc::unbounded_channel;
use url::Url;

use crate::networking::{
    crypto::SkipServerVerification,
    events::{
        ReceiveEvent,
        SendEvent
    },
    systems::{
        Connecting,
        NetworkConnections,
        SessionEventListenerState,
        receive_net_events_system,
        send_net_events_system,
        SEND_NET_EVENT_STAGE,
        RECEIVE_NET_EVENT_STAGE
    }
};

pub struct Network {
    pub addr: SocketAddr,
    pub url: Url,
    pub cert: quinn::Certificate,
    pub accept_any_cert : bool,
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
        app.add_resource::<NetworkConnections>(Default::default());

        app.add_event::<ReceiveEvent>();
        app.add_event::<SendEvent>();

        // Start a task that waits for the connection to finish opening
        tokio::spawn(
            Connecting::new(
                create_endpoint(&self.addr, &self.url, &self.cert, self.accept_any_cert).expect("Failed to create an endpoint"),
                send
            ).run()
        );

        // Add a system that consumes all network events from an MPSC and publishes them as ECS events
        app.add_system_to_stage(RECEIVE_NET_EVENT_STAGE, receive_net_events_system.system());

        // Add a system that consumes ECS events and forwards them to MPSCs which will eventually be sent over the network
        app.add_system_to_stage(SEND_NET_EVENT_STAGE, send_net_events_system.system());
    }
}

#[derive(Error, Debug)]
enum CreateEndpointError {
    #[error(transparent)]
    EndpointError(#[from] quinn::EndpointError),

    #[error(transparent)]
    ConnectError(#[from] quinn::ConnectError)
}

fn create_endpoint(
    addr: &SocketAddr,
    url: &Url,
    server_cert: &quinn::Certificate,
    accept_any_cert: bool
) -> Result<quinn::generic::Connecting<TlsSession>, CreateEndpointError>
{
    let mut client_config = ClientConfigBuilder::default();
    client_config.protocols(&[b"hq-29"]);
    
    let mut client_config = client_config.build();
    if accept_any_cert {
        let tls_cfg: &mut rustls::ClientConfig = Arc::get_mut(&mut client_config.crypto)
            .expect("Failed to get mutable reference to crypto configuration");
        
        // this is only available when compiled with "dangerous_configuration" feature
        tls_cfg
            .dangerous()
            .set_certificate_verifier(SkipServerVerification::new());
    } else {
        client_config.add_certificate_authority(server_cert.clone()).expect("Adding cert failed");
    }

    let mut endpoint = quinn::Endpoint::builder();
    
    endpoint.default_client_config(client_config);

    let (endpoint, _) = endpoint.bind(&"[::]:0".parse().expect("Failed to parse bind address"))?;
    let connecting = endpoint.connect(addr, &url.host_str().expect("Failed to get host_str from url"))?;

    Ok(connecting)
}
