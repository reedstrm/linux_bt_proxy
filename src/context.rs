pub struct ProxyContext {
    pub hostname: String,
    pub port: u16,
    pub net_mac: [u8; 6],
    pub bt_mac: [u8; 6],
    pub build_time: &'static str,
    pub version: &'static str,
}
