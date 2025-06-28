use std::sync::Arc;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use log::{info};
use anyhow::{Result};

use crate::context::ProxyContext;
use crate::utils::format_mac;

pub fn start_mdns(ctx: Arc<ProxyContext>) -> Result<()> {
    let mdns = ServiceDaemon::new().expect("Failed to create mDNS daemon");

    let mac = format_mac(&ctx.bt_mac, ":");
    let short_bt_mac = format_mac(&ctx.bt_mac[2..], "");
    let service_type = "_esphomelib._tcp.local.";
    let service_name = format!("{}_{}", ctx.hostname, short_bt_mac);
    let service_hostname = format!("{}.local.", ctx.hostname);
    let version = env!("CARGO_PKG_VERSION");

    let txt_records = [
        ("friendly_name".to_string(), format!("Bluetooth Proxy {}", &short_bt_mac)),
        ("version".to_string(), version.to_string()),
        ("mac".to_string(), mac.to_lowercase()),
        ("platform".to_string(), "linux".to_string()),
        ("network".to_string(), "ethernet".to_string()), ];

    let my_service = ServiceInfo::new(
        service_type,
        &service_name,
        &service_hostname,
        "",
        ctx.port,
        &txt_records[..],
    ).expect("Invalid service info")
    .enable_addr_auto();

    mdns.register(my_service).expect("Failed to register mDNS service");

    info!("mDNS service registered for {} on port {} with MAC {}", ctx.hostname, ctx.port, mac);
    Ok(())
}
