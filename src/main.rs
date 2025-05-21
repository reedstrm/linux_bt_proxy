use libc::{c_int, c_ushort, c_uchar, c_void, sockaddr, socket, bind, recv};
use std::mem;
use std::os::unix::io::RawFd;
use std::ptr;
use log::info;
use env_logger;

// Constants for Bluetooth HCI
const AF_BLUETOOTH: c_int = 31;
const SOCK_RAW: c_int = 3;
const BTPROTO_HCI: c_int = 1;
const HCI_CHANNEL_USER: c_ushort = 1;
const HCI_CHANNEL_MONITOR: c_ushort = 2;
const HCI_DEV_NONE: c_ushort = 0xffff;

// Define the sockaddr_hci struct
#[repr(C)]
struct SockAddrHci {
    hci_family: c_ushort,
    hci_dev: c_ushort,
    hci_channel: c_ushort,
}

fn open_hci_socket(dev_id: c_ushort, channel: c_ushort) -> Result<RawFd, std::io::Error> {
    // Create a raw socket
    let fd = unsafe { socket(AF_BLUETOOTH, SOCK_RAW, BTPROTO_HCI) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }

    // Prepare the sockaddr_hci structure
    let addr = SockAddrHci {
        hci_family: AF_BLUETOOTH as c_ushort,
        hci_dev: dev_id,
        hci_channel: channel,
    };

    // Bind the socket
    let ret = unsafe {
        bind(
            fd,
            &addr as *const _ as *const sockaddr,
            mem::size_of::<SockAddrHci>() as u32,
        )
    };
    if ret < 0 {
        return Err(std::io::Error::last_os_error());
    }

    Ok(fd)
}

fn parse_ble_advertisement(buf: &[u8]) {
    if buf.len() < 3 || buf[0] != 0x3E {
        return; // Not LE Meta Event
    }

    let plen = buf[1] as usize;
    if buf.len() < 2 + plen {
        return; // Malformed
    }

    let subevent = buf[2];
    match subevent {
        0x02 => parse_legacy_adv(&buf[3..]),
        0x0D => parse_extended_adv(&buf[3..]),
        _ => log::trace!("Unhandled LE subevent: 0x{:02X}", subevent),
    }
}

fn parse_legacy_adv(data: &[u8]) {
    if data.len() < 1 {
        return;
    }
    let num_reports = data[0];
    let mut cursor = 1;

    for _ in 0..num_reports {
        if cursor + 10 > data.len() {
            break;
        }

        let addr = &data[cursor + 2..cursor + 8];
        let data_len = data[cursor + 8] as usize;
        if cursor + 9 + data_len + 1 > data.len() {
            break;
        }

        let adv_data = &data[cursor + 9..cursor + 9 + data_len];
        let rssi = data[cursor + 9 + data_len] as i8;

        log::info!(
            "LEGACY ADV: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X} RSSI: {} dBm LEN: {}",
            addr[5], addr[4], addr[3], addr[2], addr[1], addr[0],
            rssi, adv_data.len()
        );

        cursor += 10 + data_len;
    }
}

fn parse_extended_adv(data: &[u8]) {
    if data.len() < 1 {
        return;
    }
    let num_reports = data[0];
    let mut cursor = 1;

    for _ in 0..num_reports {
        // Minimal size check: 24 bytes header + at least 1 byte payload
        if cursor + 24 > data.len() {
            break;
        }

        let addr = &data[cursor + 3..cursor + 9];
        let rssi = data[cursor + 13] as i8;
        let data_len = data[cursor + 23] as usize;

        if cursor + 24 + data_len > data.len() {
            break;
        }

        let adv_data = &data[cursor + 24..cursor + 24 + data_len];

        log::info!(
            "EXT ADV: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X} RSSI: {} dBm LEN: {}",
            addr[5], addr[4], addr[3], addr[2], addr[1], addr[0],
            rssi, adv_data.len()
        );

        cursor += 24 + data_len;
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let dev_id: c_ushort = 0; // hci0

    let fd = match open_hci_socket(dev_id, HCI_CHANNEL_USER) {
        Ok(fd) => {
            info!("Opened HCI socket in USER mode");
            fd
        }
        Err(e) => {
            info!("Failed to open USER mode: {}. Falling back to MONITOR mode.", e);
            open_hci_socket(HCI_DEV_NONE, HCI_CHANNEL_MONITOR)?
        }
    };

    let mut buf = [0u8; 1024];

    loop {
        let len = unsafe { recv(fd, buf.as_mut_ptr() as *mut c_void, buf.len(), 0) };
        if len > 0  && buf[0] == 0x03 {  // MONITOR_EVENT_PKT
            parse_ble_advertisement(&buf[6..len as usize]);
        }

    }
}
