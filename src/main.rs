mod api;
mod ble;
mod context;
mod handlers;
mod mdns;
mod proto;
mod server;
mod utils;

use clap::Parser;
use gethostname::gethostname;
use log::{info, warn};
use mac_address::get_mac_address;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::context::ProxyContext;
use crate::utils::parse_mac;

fn default_hostname() -> String {
    gethostname().to_string_lossy().into_owned()
}

#[derive(Parser, Debug)]
#[command(name = "linux_bt_proxy")]
#[command(about = "Bluetooth Proxy Daemon for ESPHome", long_about = None)]
struct Cli {
    /// Bluetooth adapter index (e.g. 0 for hci0)
    #[arg(short = 'a', long, default_value_t = 0)]
    hci: u16,

    /// TCP listen address (default: 0.0.0.0:6053)
    #[arg(short, long, default_value = "0.0.0.0:6053")]
    listen: SocketAddr,

    /// Hostname to advertise (default: system hostname)
    #[arg(long, default_value_t = default_hostname())]
    hostname: String,

    /// MAC address for mDNS
    #[arg(short, long, value_parser = parse_mac)]
    mac: Option<[u8; 6]>,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    let mac: [u8; 6] = match cli.mac {
        Some(mac) => mac,
        None => match get_mac_address() {
            Ok(Some(mac)) => mac.bytes(),
            Ok(None) => {
                log::warn!("System has no available MAC address.");
                eprintln!("Fatal: No MAC address provided via CLI or available on system.");
                std::process::exit(1);
            }
            Err(e) => {
                log::error!("Error while getting MAC address: {}", e);
                eprintln!("Fatal: Could not determine MAC address.");
                std::process::exit(1);
            }
        },
    };

    let bt_mac =
        utils::get_bt_mac(cli.hci).expect(&format!("Can't get Bluetooth MAC for hci{}", cli.hci));

    let ctx = Arc::new(ProxyContext {
        hostname: cli.hostname,
        port: cli.listen.port(),
        net_mac: mac,
        bt_mac: bt_mac,
        build_time: env!("BUILD_TIME"),
        version: env!("CARGO_PKG_VERSION"),
    });


    let (tx, rx) = broadcast::channel(100);

    // first cut: use bluez stack, ask for active scanning
    tokio::spawn(ble::run_bluez_advertisement_listener(cli.hci, tx.clone()));

    info!("Listening for ble advertisements on hci{}", cli.hci);

    let _mdns_service = mdns::start_mdns(ctx.clone()).unwrap_or_else(|e| {
        warn!("Critical error: failed to register mDNS service: {}", e);
        std::process::exit(1);
    });

    info!("mDNS service registered");
    
    let _ = server::run_tcp_server(ctx.clone(), cli.listen, rx).await;

    Ok(())
}
