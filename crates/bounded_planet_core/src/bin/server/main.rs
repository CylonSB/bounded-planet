use std::{fs, net::SocketAddr, path::PathBuf, sync::Arc, time::SystemTime};
use std::time::Duration;

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use structopt::StructOpt;
use tracing::{Level, info, error};

use bounded_planet::networking::{components::Connection, events::ReceiveEvent, events::SendEvent, packets::{Packet, Ping, Pong, StreamType}, systems::NetEventLoggerState, systems::log_net_events};

#[derive(StructOpt, Debug)]
#[structopt(name = "server")]
struct Opt {
    /// Address to listen on
    #[structopt(long = "listen", default_value = "[::1]:4433")]
    addr: SocketAddr,

    /// TLS private key in PEM format
    #[structopt(parse(from_os_str), short = "k", long = "key", required=true)]
    key: PathBuf,
    
    /// TLS certificate in PEM format
    #[structopt(parse(from_os_str), short = "c", long = "cert", required=true)]
    cert: PathBuf,
}

fn main() {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(Level::DEBUG)
            .finish(),
    )
    .expect("Failed to configure logging");

    let opt = Opt::from_args();
    let code = {
        if let Err(e) = run(opt) {
            error!("ERROR: {}", e);
            1
        } else {
            0
        }
    };
    std::process::exit(code);
}

#[tokio::main]
async fn run(options: Opt) -> Result<(), Box<dyn std::error::Error>> {
    // Create a Bevy app
    let mut app = App::build();
    app.add_plugin(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
        1.0 / 10.0,
    )));

    let (key, cert) = get_certs(&options.key, &options.cert)?;
    app.add_plugin(bounded_planet::networking::server::plugin::Network {
        certificate: cert,
        private_key: key,
        addr: options.addr,
    });

    app.add_system(send_pings.system());

    app.add_resource(PongLoggerState::default());
    app.add_system(log_pongs.system());

    app.add_resource(NetEventLoggerState::default());
    app.add_system(log_net_events.system());

    // Run it forever
    app.run();

    Ok(())
}

/// Fetch certificates to use
fn get_certs(key_path: &PathBuf, cert_path: &PathBuf) -> Result<(quinn::PrivateKey, quinn::CertificateChain), Box<dyn std::error::Error>> {

    info!("Loading Key: {:?}", key_path);
    let key = fs::read(key_path)?;
    let key = if key_path.extension().map_or(false, |x| x == "der") {
        quinn::PrivateKey::from_der(&key)?
    } else {
        quinn::PrivateKey::from_pem(&key)?
    };

    info!("Loading Cert: {:?}", cert_path);
    let cert_chain = fs::read(cert_path)?;
    let cert_chain = if cert_path.extension().map_or(false, |x| x == "der") {
        quinn::CertificateChain::from_certs(quinn::Certificate::from_der(&cert_chain))
    } else {
        quinn::CertificateChain::from_pem(&cert_chain)?
    };

    Ok((
        key,
        cert_chain,
    ))
}

fn send_pings(mut sender: ResMut<Events<SendEvent>>, conn: &Connection) {
    sender.send(SendEvent::SendPacket {
        connection: conn.id,
        stream: StreamType::PingPong,
        data: Arc::new(
            Packet::Ping(Ping::default())
        ),
    });
}

#[derive(Default)]
pub struct PongLoggerState {
    pub event_reader: EventReader<ReceiveEvent>,
}

fn log_pongs(mut state: ResMut<PongLoggerState>, receiver: ResMut<Events<ReceiveEvent>>) {
    for evt in state.event_reader.iter(&receiver) {
        if let ReceiveEvent::ReceivedPacket { data, .. } = evt {
            if let Packet::Pong(Pong { timestamp }) = **data {
                
                let time_sent = SystemTime::UNIX_EPOCH.checked_add(
                    Duration::from_millis(timestamp as u64)
                ).expect("Overflowed SystemTime");

                let time_now = SystemTime::now();

                let latency = time_now.duration_since(time_sent);

                info!("Received Pong. Latency {:?}", latency);
            }
        }
    }
}