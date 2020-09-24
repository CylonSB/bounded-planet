use std::{fs, net::ToSocketAddrs, path::PathBuf, time::Duration};

use structopt::StructOpt;
use bevy::{app::ScheduleRunnerPlugin, prelude::App};

use tracing::{Level, info};
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
    .unwrap();

    // Resolve URL from options
    let url = options.url;
    let remote = (url.host_str().unwrap(), url.port().unwrap_or(4433))
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| panic!("couldn't resolve to an address")).unwrap();

    // Create a Bevy app
    let mut app = App::build();
    app.add_plugin(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
        1.0 / 10.0,
    )));

    let cert = get_cert(&options.cert)?;
    app.add_plugin(bounded_planet::networking::client::plugin::Network {
        addr: remote,
        url: url,
        cert: cert
    });

    // Run it forever
    app.run();

    Ok(())
}

// Fetch vertificates to use
fn get_cert(cert_path: &PathBuf) -> Result<quinn::CertificateChain, Box<dyn std::error::Error>> {

    info!("Loading Cert: {:?}", cert_path);
    let cert_chain = fs::read(cert_path)?;
    let cert_chain = if cert_path.extension().map_or(false, |x| x == "der") {
        quinn::CertificateChain::from_certs(quinn::Certificate::from_der(&cert_chain))
    } else {
        quinn::CertificateChain::from_pem(&cert_chain)?
    };

    Ok(cert_chain)
}