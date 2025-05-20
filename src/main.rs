use rumble::bluez::manager::Manager;
use rumble::api::{Central, Peripheral};
use std::time::Duration;
use std::thread;
use log::info;
use env_logger;
use clap::Parser;
use gethostname::gethostname;

/// Bluetooth proxy CLI
#[derive(Parser, Debug)]
#[command(name = "linux_bt_proxy")]
#[command(about = "Linux Bluetooth Proxy for ESPHome", long_about = None)]
struct Args {
    /// HCI interface to use (e.g., hci0)
    #[arg(long, default_value = "hci0")]
    hci: String,

    /// Hostname to advertise via mDNS
    #[arg(long)]
    hostname: Option<String>,

    /// MAC address to advertise
    #[arg(long)]
    mac: Option<String>,

    /// Network interfaces to advertise on (comma-separated)
    #[arg(long)]
    interfaces: Option<String>,
}

fn main() {
    env_logger::init();
    let args = Args::parse();

    let hostname = args
        .hostname
        .unwrap_or_else(|| gethostname().to_string_lossy().into_owned());

    let manager = Manager::new().expect("Failed to initialize BLE manager");

    // Find the adapter that matches the requested HCI interface
    let adapters = manager.adapters().expect("Unable to retrieve adapters");
    let adapter_info = adapters
        .iter()
        .find(|a| a.name == args.hci)
        .expect("Specified HCI interface not found");

    let adapter = manager.down(adapter_info).expect("Failed to bring adapter down");
    let adapter = manager.up(&adapter).expect("Failed to bring adapter up");
    let central = adapter.connect().expect("Failed to connect to adapter");

    // Determine MAC address to advertise
    let adapter_mac = args.mac.unwrap_or_else(|| adapter_info.mac_address.to_string());
    info!("Using MAC address: {}", adapter_mac);

    central.start_scan().expect("Failed to start scanning");

    thread::sleep(Duration::from_secs(5));
    let peripherals = central.peripherals();

    for peripheral in peripherals {
        let properties = peripheral.properties();
        info!("Discovered device: {:?}", properties);
    }

    info!(
        "Advertising as '{}' on interfaces: {:?}",
        hostname,
        args.interfaces.as_deref().unwrap_or("all")
    );
}
