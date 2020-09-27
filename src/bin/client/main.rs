use std::{fs, net::ToSocketAddrs, path::PathBuf, time::Duration};

use structopt::StructOpt;
use bevy::prelude::*;
use bevy::{app::ScheduleRunnerPlugin};

use bounded_planet::networking::components::Connection;
use tracing::{Level, info, warn};
use url::Url;

#[derive(StructOpt, Debug)]
#[structopt(name = "client")]
struct Opt {
    /// Address to connect to
    #[structopt(long="url", default_value="[::1]:4433")]
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
            .with_max_level(Level::TRACE)
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

    app.add_system(log_connections.system());

    // Run it forever
    app.run();

    Ok(())
}

/// Fetch certificates to use
fn get_cert(cert_path: &PathBuf) -> Result<quinn::Certificate, Box<dyn std::error::Error>> {
    info!("Loading Cert: {:?}", cert_path);
    Ok(quinn::Certificate::from_der(&fs::read(cert_path)?)?)
}

fn log_connections(_conn: &Connection) {
    warn!("Connection Entity Exists!");
}
