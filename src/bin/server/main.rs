use std::net::SocketAddr;
use std::sync::Arc;

use structopt::StructOpt;

use futures::{StreamExt, TryFutureExt};

use tokio::net::TcpListener;
use tokio::prelude::*;

use tracing::{error, info, info_span};

use rcgen::generate_simple_self_signed;

#[derive(StructOpt, Debug)]
#[structopt(name = "server")]
struct Opt {

    /// Address to listen on
    #[structopt(long = "listen", default_value = "[::1]:4433")]
    listen: SocketAddr,
}

fn main() {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .finish(),
    )
    .unwrap();
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

    // Configure endpoint
    let mut transport_config = quinn::TransportConfig::default();
    transport_config.stream_window_uni(128);
    let mut server_config = quinn::ServerConfig::default();
    server_config.transport = Arc::new(transport_config);
    let mut server_config = quinn::ServerConfigBuilder::new(server_config);
    server_config.protocols(&[b"hq-29"]);

    // Configure encryption
    let (priv_key, cert) = get_certs()?;
    server_config.certificate(quinn::CertificateChain::from_certs(vec![cert]), priv_key)?;

    // Begin listening for connections
    let mut endpoint = quinn::Endpoint::builder();
    endpoint.listen(server_config.build());
    let (endpoint, mut incoming) = endpoint.bind(&options.listen)?;
    drop(endpoint);

    while let Some(conn) = incoming.next().await {
        info!("connection incoming");
        tokio::spawn(
            handle_connection(conn).unwrap_or_else(move |e| {
                error!("connection failed: {reason}", reason = e.to_string())
            }),
        );
    }

    Ok(())
}

async fn handle_connection(conn: quinn::Connecting) -> Result<(), Box<dyn std::error::Error>> {
    info!("Connection :O");
    Ok(())
}

fn get_certs() -> Result<(quinn::PrivateKey, quinn::Certificate), Box<dyn std::error::Error>> {
    info!("generating self-signed certificate");
    let cert = generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let key = cert.serialize_private_key_der();
    let cert = cert.serialize_der().unwrap();
    Ok((quinn::PrivateKey::from_der(&key)?, quinn::Certificate::from_der(&cert)?))
}