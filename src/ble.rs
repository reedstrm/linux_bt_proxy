use libc::{c_int, c_void, socket, bind, sockaddr, AF_BLUETOOTH, SOCK_RAW, recv};
use std::mem::{size_of, zeroed};

use std::os::fd::RawFd;
use tokio::io::unix::AsyncFd;
use tokio::sync::broadcast::Sender;
use log::{debug, info};
use zbus::{Connection, Proxy};
use zbus::zvariant::Value;

use crate::api::api::BluetoothLERawAdvertisement;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct sockaddr_hci {
    hci_family: libc::sa_family_t,
    hci_dev: u16,
    hci_channel: u16,
}

const BTPROTO_HCI: c_int = 1;
pub const HCI_CHANNEL_USER: u16 = 1;
pub const HCI_CHANNEL_MONITOR: u16 = 2;
pub const HCI_DEV_NONE: u16 = 0xffff;

pub async fn run_hci_monitor_async(fd: RawFd, tx: Sender<BluetoothLERawAdvertisement>) -> std::io::Result<()> {
    let async_fd = AsyncFd::new(fd)?;

    loop {
        let mut guard = async_fd.readable().await?;
        match guard.try_io(|_| Ok(read_hci_packet(fd))) {
            Ok(Ok(advs)) => {
                for adv in advs {
                    let _ = tx.send(adv);
                }
            }
            Ok(Err(e)) => {
                debug!("Read error: {:?}", e);
            }
            Err(e) => {
                debug!("Async readiness error: {:?}", e);
            }
        }
    }
}

fn read_hci_packet(fd: RawFd) -> Vec<BluetoothLERawAdvertisement> {
    let mut buf = [0u8; 1024];
    let len = unsafe { recv(fd, buf.as_mut_ptr() as *mut c_void, buf.len(), 0) } as usize;
    // debug!("Raw packet header: {:02x?}", &buf[..16]);
    if len < 7 || buf[0] != 0x03 {
        return vec![];
    }

    let evt = &buf[6..len];
    if evt.get(0) != Some(&0x3E) {
        return vec![];
    }

    parse_ble_advertisement(evt)
}

fn parse_ble_advertisement(data: &[u8]) -> Vec<BluetoothLERawAdvertisement> {
    if data.len() < 3 {
        return vec![];
    }

    match data[2] {
        0x02 => parse_legacy_adv(&data[3..]),
        0x0D => parse_extended_adv(&data[3..]),
        _ => vec![],
    }
}

fn parse_extended_adv(data: &[u8]) -> Vec<BluetoothLERawAdvertisement> {
    let mut ads = Vec::new();

    if data.len() < 1 {
        return ads;
    }

    let num_reports = data[0];
    let mut cursor = 1;

    for _ in 0..num_reports {
        if cursor + 24 > data.len() {
            break;
        }

        let addr = &data[cursor + 3..cursor + 9];
        let addr_type = data[cursor + 2];
        let rssi = data[cursor + 13] as i8;
        let data_len = data[cursor + 23] as usize;

        if cursor + 24 + data_len > data.len() {
            break;
        }

        let adv_data = &data[cursor + 24..cursor + 24 + data_len];

        debug!(
        "EXT ADV: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X} RSSI: {} dBm LEN: {}",
        addr[5], addr[4], addr[3], addr[2], addr[1], addr[0], rssi, adv_data.len()
        );

        ads.push(BluetoothLERawAdvertisement {
            address: bdaddr_to_u64(addr),
            rssi: rssi as i32,
            address_type: addr_type as u32,
            data: adv_data.to_vec(),
            ..Default::default()
        });

        cursor += 24 + data_len;
    }

    ads
}

fn parse_legacy_adv(data: &[u8]) -> Vec<BluetoothLERawAdvertisement> {
    let mut ads = Vec::new();

    if data.len() < 1 {
        return ads;
    }

    let num_reports = data[0];
    let mut cursor = 1;

    for _ in 0..num_reports {
        if cursor + 10 > data.len() {
            break;
        }

        let addr_type = data[cursor];
        let addr = &data[cursor + 1..cursor + 7];
        let data_len = data[cursor + 7] as usize;

        if cursor + 8 + data_len + 1 > data.len() {
            break;
        }

        let adv_data = &data[cursor + 8..cursor + 8 + data_len];
        let rssi = data[cursor + 8 + data_len] as i8;

        debug!(
        "LEG ADV: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X} RSSI: {} dBm LEN: {}",
        addr[5], addr[4], addr[3], addr[2], addr[1], addr[0], rssi, adv_data.len()
        );

        ads.push(BluetoothLERawAdvertisement {
            address: bdaddr_to_u64(addr),
            rssi: rssi as i32,
            address_type: addr_type as u32,
            data: adv_data.to_vec(),
            ..Default::default()
        });

        cursor += 9 + data_len;
    }

    ads
}

fn bdaddr_to_u64(addr: &[u8]) -> u64 {
    addr.iter().rev().fold(0u64, |acc, &b| (acc << 8) | b as u64)
}

pub fn open_hci_socket(dev_id: u16) -> std::io::Result<RawFd> {
    let try_open = |dev_id: u16, channel: u16| -> std::io::Result<RawFd> {
        let fd = unsafe { socket(AF_BLUETOOTH, SOCK_RAW, BTPROTO_HCI) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }

        let mut addr: sockaddr_hci = unsafe { zeroed() };
        addr.hci_family = AF_BLUETOOTH as u16;
        addr.hci_dev = dev_id;
        addr.hci_channel = channel;

        let ret = unsafe {
            bind(
                fd,
                &addr as *const _ as *const sockaddr,
                size_of::<sockaddr_hci>() as u32,
            )
        };
        if ret < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(fd)
        }
    };

    match try_open(dev_id, HCI_CHANNEL_USER) {
        Ok(fd) => {
            info!("Opened HCI socket in USER mode");
            Ok(fd)
        }
        Err(e) => {
            info!("Failed to open USER mode: {}. Falling back to MONITOR mode.", e);
            try_open(HCI_DEV_NONE, HCI_CHANNEL_MONITOR)
        }
    }
}

/// Starts discovery on hci via D-Bus, only if not already scanning.
pub async fn ensure_scanning_enabled( hci: u16) -> zbus::Result<zbus::Connection> {
    let path = format!("/org/bluez/hci{}", hci);

    let conn = Connection::system().await?;
    let proxy = Proxy::new(
        &conn,
        "org.bluez",
        path,
        "org.bluez.Adapter1",
    )
    .await?;

    let discovering: bool = proxy
        .get_property::<Value>("Discovering")
        .await?
        .downcast()
        .expect("Discovering property has unexpected type");

    if discovering {
        log::info!("Adapter is already scanning (via D-Bus).");
    } else {
        log::info!("Starting Bluetooth discovery via D-Bus...");
        proxy.call_method("StartDiscovery", &()).await?;
    }

    Ok(conn)
}

