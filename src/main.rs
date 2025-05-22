mod api;
mod ble;
mod proto;
mod mdns;
mod server;

use clap::Parser;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::sync::broadcast;
use log::info;
use env_logger::Env;


/// ESPHome-compatible Bluetooth proxy daemon
#[derive(Parser, Debug)]
#[command(name = "linux_bt_proxy")]
struct Args {
    /// HCI adapter index (e.g. 0 for hci0)
    #[arg(long, default_value_t = 0)]
    hci: usize,

    /// Hostname to advertise over mDNS
    #[arg(long)]
    hostname: Option<String>,

    /// Fake MAC address to advertise
    #[arg(long)]
    mac: Option<String>,

    /// Interfaces to advertise on (optional)
    #[arg(long)]
    interfaces: Vec<String>,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let hostname = gethostname::gethostname().to_string_lossy().into_owned();
    let mac = "00:11:22:33:44:55"; // replace with dynamic MAC if desired
    let port = 6053;

    let _mdns_service = mdns::start_mdns(&hostname, mac, port);
    info!("mDNS service registered");

    let (tx, rx) = broadcast::channel(100);

    let hci_fd = ble::open_hci_socket().unwrap_or_else(|e| {
        eprintln!("Failed to open bluetooth device, exiting: {:?}", e);
        std::process::exit(1);
    });


    let ble_task = tokio::spawn(ble::run_hci_monitor_async_with_tx(hci_fd, tx));
    let tcp_task = tokio::spawn(server::run_tcp_server(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port),
        rx,
    ));

    let _ = tokio::try_join!(ble_task, tcp_task)?;
    Ok(())
}

