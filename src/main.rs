use clap::Parser;
use log::{error, info};
use rumble::bluez::manager::Manager;
use rumble::api::{Central, CentralEvent};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread;
use std::net::{TcpListener, TcpStream};
use std::io::Write;
use prost::Message;
use bytes::BytesMut;
use libmdns::Responder;
use std::process::exit;
use gethostname::gethostname;

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

mod api {
    include!(concat!(env!("OUT_DIR"), "/api.rs"));
}

fn format_mac(mac: &[u8]) -> String {
    mac.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(":")
}

fn start_mdns_service(hostname: &str, port: u16, mac: &str) {
    let responder = Responder::new().expect("Failed to start mDNS responder");
    let _svc = responder.register(
        "_esphomelib._tcp".into(),
        format!("{}._esphomelib._tcp.local.", hostname),
        port,
        &[("mac", mac.replace(":", "").to_lowercase().as_str())],
    );

    info!("mDNS service registered for {} on port {} with MAC {}", hostname, port, mac);
    std::mem::forget(responder);
}

fn handle_client(stream: TcpStream, clients: Arc<Mutex<Vec<TcpStream>>>) {
    let mut clients = clients.lock().unwrap();
    clients.push(stream.try_clone().unwrap());
    drop(clients);
    info!("Client connected: {}", stream.peer_addr().unwrap());
}

fn start_server(clients: Arc<Mutex<Vec<TcpStream>>>, port: u16) {
    let listener = TcpListener::bind(("0.0.0.0", port)).expect("Failed to bind TCP server");
    info!("TCP server listening on port {}", port);
    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            let clients = clients.clone();
            thread::spawn(move || handle_client(stream, clients));
        }
    }
}

fn main() {
    env_logger::init();
    let args = Args::parse();

    let manager = Manager::new().unwrap();
    let adapters = manager.adapters().unwrap();

    if args.hci >= adapters.len() {
        error!("Invalid HCI index: {} (only {} adapter(s) found)", args.hci, adapters.len());
        exit(1);
    }

    let adapter = adapters[args.hci].connect().unwrap();
    let adapter_mac = adapter.adapter_info().unwrap();
    let hostname = args.hostname.unwrap_or_else(|| gethostname::gethostname().to_string_lossy().into_owned());
    let fake_mac = args.mac.unwrap_or_else(|| adapter_mac.clone());

    info!("Using HCI{} - MAC: {}", args.hci, adapter_mac);
    info!("Hostname: {}", hostname);
    info!("Advertised MAC: {}", fake_mac);

    start_mdns_service(&hostname, 6053, &fake_mac);

    let clients: Arc<Mutex<Vec<TcpStream>>> = Arc::new(Mutex::new(Vec::new()));
    let server_clients = clients.clone();
    thread::spawn(move || start_server(server_clients, 6053));

    adapter.start_scan().expect("Failed to start scan");
    info!("Started BLE scanning");

    loop {
        if let Ok(event) = adapter.event_receiver().recv_timeout(Duration::from_secs(2)) {
            if let CentralEvent::DeviceDiscovered(d) = event {
                if let Ok(properties) = adapter.device(&d).and_then(|dev| dev.properties()) {
                    let addr_u64 = properties.address.iter().fold(0u64, |acc, &b| (acc << 8) | b as u64);
                    let rssi = properties.rssi.unwrap_or(0);
                    let data = properties.advertisement_data;

                    let mut msg = api::BluetoothLeRawAdvertisement {
                        address: addr_u64,
                        rssi: rssi as i32,
                        address_type: 0,
                        data: data.clone().into(),
                    };

                    let mut buf = BytesMut::with_capacity(128);
                    msg.encode(&mut buf).unwrap();

                    let mut packet = vec![0x33];
                    packet.extend_from_slice(&(buf.len() as u32).to_be_bytes());
                    packet.extend_from_slice(&buf);

                    let mut clients = clients.lock().unwrap();
                    clients.retain(|c| c.write_all(&packet).is_ok());
                }
            }
        }
    }
}
