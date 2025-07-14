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

    /// TCP listen address (default: [::]:6053)
    #[arg(short, long, default_value = "[::]:6053")]
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

    // Validate HCI adapter exists before proceeding
    if utils::get_bt_mac(cli.hci).is_none() {
        log::error!(
            "Bluetooth adapter hci{} does not exist or is not accessible",
            cli.hci
        );
        log::error!("Fatal: Check available adapters with 'hciconfig' or 'bluetoothctl list'");
        std::process::exit(1);
    }

    let mac: [u8; 6] = match cli.mac {
        Some(mac) => mac,
        None => match get_mac_address() {
            Ok(Some(mac)) => mac.bytes(),
            Ok(None) => {
                log::warn!("System has no available MAC address.");
                log::error!("Fatal: No MAC address provided via CLI or available on system.");
                std::process::exit(1);
            }
            Err(e) => {
                log::error!("Error while getting MAC address: {e}");
                log::error!("Fatal: Could not determine MAC address.");
                std::process::exit(1);
            }
        },
    };

    let bt_mac = match utils::get_bt_mac(cli.hci) {
        Some(mac) => mac,
        None => {
            log::error!("Failed to get Bluetooth MAC for hci{}", cli.hci);
            log::error!("Fatal: Cannot access Bluetooth adapter hci{}. Check if it exists and is accessible.", cli.hci);
            std::process::exit(1);
        }
    };

    let ctx = Arc::new(ProxyContext {
        hostname: cli.hostname,
        port: cli.listen.port(),
        net_mac: mac,
        bt_mac,
        build_time: env!("BUILD_TIME"),
        version: env!("CARGO_PKG_VERSION"),
    });

    let (tx, rx) = broadcast::channel(100);

    // first cut: use bluez stack, ask for active scanning
    let mut ble_handle = tokio::spawn(ble::run_bluez_advertisement_listener(cli.hci, tx.clone()));

    // Check if BLE listener started successfully
    tokio::select! {
        _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
            // BLE listener is still running after 100ms, assume it started successfully
        }
        result = &mut ble_handle => {
            match result {
                Ok(Err(e)) => {
                    log::error!("Failed to start BLE advertisement listener: {e}");
                    log::error!("Fatal: Cannot connect to BlueZ D-Bus service. Check if bluetoothd is running.");
                    std::process::exit(1);
                }
                Err(e) => {
                    log::error!("BLE advertisement listener task panicked: {e}");
                    log::error!("Fatal: Critical error in BLE listener.");
                    std::process::exit(1);
                }
                Ok(Ok(())) => {
                    log::error!("BLE advertisement listener exited unexpectedly");
                    log::error!("Fatal: BLE listener stopped.");
                    std::process::exit(1);
                }
            }
        }
    }

    info!("Listening for ble advertisements on hci{}", cli.hci);

    mdns::start_mdns(ctx.clone()).unwrap_or_else(|e| {
        warn!("Critical error: failed to register mDNS service: {e}");
        std::process::exit(1);
    });

    info!("mDNS service registered");

    if let Err(e) = server::run_tcp_server(ctx.clone(), cli.listen, rx).await {
        log::error!("TCP server error: {e}");
        log::error!("Fatal: TCP server failed to start or crashed.");
        std::process::exit(1);
    }

    Ok(())
}
