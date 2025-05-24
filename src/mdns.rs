use mdns_sd::{ServiceDaemon, ServiceInfo};
use log::{info};

pub fn start_mdns(hostname: &str, bt_mac: &str, mac: &str, port: u16) -> std::io::Result<()> {
    let mdns = ServiceDaemon::new().expect("Failed to create mDNS daemon");

    let stripped_bt_mac = bt_mac.replace(":", "").to_lowercase();
    let short_bt_mac = &stripped_bt_mac[stripped_bt_mac.len()-6..];
    let service_type = "_esphomelib._tcp.local.";
    let service_name = format!("{}_{}", hostname, short_bt_mac);
    let service_hostname = format!("{}.local.", hostname);

    let txt_records = [
        ("friendly_name".to_string(), format!("Bluetooth Proxy {}", &short_bt_mac)),
        ("version".to_string(), "0.1".to_string()),
        ("mac".to_string(), mac.to_lowercase()),
        ("platform".to_string(), "linux".to_string()),
        ("network".to_string(), "ethernet".to_string()), ];

    let my_service = ServiceInfo::new(
        service_type,
        &service_name,
        &service_hostname,
        "",
        port,
        &txt_records[..],
    ).expect("Invalid service info")
    .enable_addr_auto();

    mdns.register(my_service).expect("Failed to register mDNS service");

    info!("mDNS service registered for {} on port {} with MAC {}", hostname, port, mac);
    Ok(())
}
