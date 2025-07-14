use anyhow::Result;
use libc::{self, c_int, c_uint, c_ulong, c_ushort, c_void};
use std::mem::zeroed;

pub fn get_bt_mac(hci_index: u16) -> Option<[u8; 6]> {
    // These are constants from BlueZ / Bluetooth headers
    const AF_BLUETOOTH: c_int = 31;
    const SOCK_RAW: c_int = 3;
    const BTPROTO_HCI: c_int = 1;
    const HCIGETDEVINFO: c_ulong = 0x800448d3; // actual ioctl value (same across systems)

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    struct HciDevInfo {
        dev_id: c_ushort,
        name: [u8; 8],
        bdaddr: [u8; 6],
        flags: c_uint,
        _padding: [u8; 84], // full size = 104
    }

    log::debug!("Attempting to open HCI device hci{hci_index}");

    let sock = unsafe { libc::socket(AF_BLUETOOTH, SOCK_RAW, BTPROTO_HCI) };
    if sock < 0 {
        log::error!("Failed to open raw HCI socket: errno {sock}");
        return None;
    }

    let mut devinfo: HciDevInfo = unsafe { zeroed() };
    devinfo.dev_id = hci_index;

    let ret = unsafe { libc::ioctl(sock, HCIGETDEVINFO, &mut devinfo as *mut _ as *mut c_void) };

    unsafe {
        libc::close(sock);
    }

    if ret < 0 {
        log::error!("ioctl HCIGETDEVINFO failed for hci{hci_index} (ret = {ret})");
        return None;
    }

    // linux bluetooth devices store mac little-endian. Need big-endian for protocols
    let mac: [u8; 6] = devinfo
        .bdaddr
        .iter()
        .rev()
        .cloned()
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    log::info!(
        "Retrieved MAC for hci{}: {}",
        hci_index,
        format_mac(&mac, ":")
    );
    Some(mac)
}

pub fn format_mac(mac: &[u8], sep: &str) -> String {
    mac.iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(sep)
}

pub fn parse_mac(s: &str) -> Result<[u8; 6], String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 6 {
        return Err("Invalid MAC format: expected 6 hex bytes separated by ':'".to_string());
    }

    let mut mac = [0u8; 6];
    for (i, part) in parts.iter().enumerate() {
        mac[i] = u8::from_str_radix(part, 16).map_err(|_| format!("Invalid hex byte: '{part}'"))?;
    }
    Ok(mac)
}
