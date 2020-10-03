use std::{fs, net::ToSocketAddrs, path::PathBuf, sync::Arc, time::Duration};

use structopt::StructOpt;
use bevy::prelude::*;
use bevy::app::ScheduleRunnerPlugin;

use bounded_planet::networking::{
    systems::{NetEventLoggerState, log_net_events},
    events::{ReceiveEvent, SendEvent},
    packets::{Packet, Ping, Pong, StreamType}
};
use tracing::{Level, info};
use url::Url;

#[derive(StructOpt, Debug)]
#[structopt(name = "client")]
struct Opt {
    /// Address to connect to
    #[structopt(long="url", default_value="quic://localhost:4433")]
    url: Url,

    /// TLS certificate in PEM format
    #[structopt(parse(from_os_str), short="c", long="cert")]
    cert: PathBuf,

    #[structopt(short="a", long="accept_any")]
    accept_any_cert: bool
}

fn main() {
    let opt = Opt::from_args();
    let code = {
        if let Err(e) = run(opt) {
            eprintln!("ERROR: {}", e);
            1
        } else {
            0
        }
    };
    ::std::process::exit(code);
}

#[tokio::main]
async fn run(options: Opt) -> Result<(), Box<dyn std::error::Error>> {

    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(Level::DEBUG)
            .finish(),
    )
    .expect("Failed to configure logging");

    // Resolve URL from options
    let url = options.url;
    let remote = (url.host_str().expect("Failed to get host string from URL"), url.port().unwrap_or(4433))
        .to_socket_addrs()?
        .next()
        .expect("couldn't resolve to an address");
 
    // Create a Bevy app
    let mut app = App::build();
    app.add_plugin(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
        1.0 / 10.0,
    )));

    let cert = get_cert(&options.cert)?;
    app.add_plugin(bounded_planet::networking::client::plugin::Network {
        addr: remote,
        url,
        cert,
        accept_any_cert: options.accept_any_cert
    });

    app.init_resource::<PingResponderState>();
    app.add_system(respond_to_pings.system());

    app.init_resource::<NetEventLoggerState>();
    app.add_system(log_net_events.system());

    // Run it forever
    app.run();

    Ok(())
}

/// Fetch certificates to use
fn get_cert(cert_path: &PathBuf) -> Result<quinn::Certificate, Box<dyn std::error::Error>> {
    info!("Loading Cert: {:?}", cert_path);
    Ok(quinn::Certificate::from_der(&fs::read(cert_path)?)?)
}

#[derive(Default)]
pub struct PingResponderState {
    pub event_reader: EventReader<ReceiveEvent>,
}
   

fn respond_to_pings(
    mut state: ResMut<PingResponderState>,
    receiver: ResMut<Events<ReceiveEvent>>,
    mut sender: ResMut<Events<SendEvent>>,
) {
    for evt in state.event_reader.iter(&receiver) {
        if let ReceiveEvent::ReceivedPacket { ref connection, data } = evt {
            if let Packet::Ping(Ping { timestamp }) = **data {
                sender.send(SendEvent::SendPacket {
                    connection: *connection,
                    stream: StreamType::PingPong,
                    data: Arc::new(Packet::Pong(Pong { timestamp }))
                });
                info!("Received Ping, sending pong. {:?}", connection);
            }
        }
    }
}
