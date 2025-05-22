mod api;
mod ble;
mod mdns;
mod proto;
mod server;

use clap::Parser;
use mac_address::get_mac_address;
use gethostname::gethostname;
use std::net::SocketAddr;
use tokio::sync::broadcast;
use log::{info, warn};

fn default_mac() -> String {
    match get_mac_address() {
        Ok(Some(ma)) => ma.to_string(),
        _ => "00:11:22:33:44:55".to_string(), // Fallback
    }
}

fn default_hostname() -> String {
    gethostname().to_string_lossy().into_owned()
}

#[derive(Parser, Debug)]
#[command(name = "linux_bt_proxy")]
#[command(about = "Bluetooth Proxy Daemon for ESPHome", long_about = None)]
struct Cli {
     /// HCI adapter index (e.g. 0 for hci0)
     #[arg(long, default_value_t = 0)]
     hci: u16,

    /// TCP listen address (default: 0.0.0.0:6053)
    #[arg(short, long, default_value = "0.0.0.0:6053")]
    listen: SocketAddr,

    /// Hostname to advertise (default: system hostname)
    #[arg(long, default_value_t = default_hostname())]
    hostname: String,

    /// MAC address for mDNS (default: system interface)
    #[arg(short, long, default_value_t = default_mac())]
    mac: String,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    let _mdns_service = mdns::start_mdns(&cli.hostname, &cli.mac, cli.listen.port()).unwrap_or_else(|e| {
    warn!("Critical error: failed to register mDNS service: {}", e);
    std::process::exit(1);
});

    info!("mDNS service registered");

    let (tx, rx) = broadcast::channel(100);

    // Open Bluetooth device
    let hci_fd = match ble::open_hci_socket(cli.hci) {
        Ok(fd) => fd,
        Err(e) => {
            warn!("Failed to open bluetooth device: {:?}", e);
            std::process::exit(1);
        }
    };

    tokio::spawn(server::run_tcp_server(cli.listen, rx));

    ble::run_hci_monitor_async(hci_fd, tx).await?;

    Ok(())
}
