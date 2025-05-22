use libmdns::{Responder, Service};
use std::net::Ipv4Addr;
use std::mem;
use log::{debug, info};

pub fn start_mdns(hostname: &str, mac: &str, port: u16) -> Service {
    let responder = Responder::new().expect("Failed to create mDNS responder");
    let service_name = format!("{}_{}", hostname, mac.replace(":", "").to_lowercase());

    responder.register(
        "_esphomelib._tcp".to_string(),
        service_name.clone(),
        port,
        &[],
    );

    info!("mDNS service registered for {} on port {} with MAC {}", hostname, port, mac);
    std::mem::forget(responder);
}
