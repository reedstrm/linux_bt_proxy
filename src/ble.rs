use libc::{c_int, c_ushort, c_uchar, c_void, sockaddr, socket, bind, recv};
//-use std::mem;
//-use std::os::unix::io::RawFd;
//-use std::ptr;
//-use env_logger;

use std::os::fd::RawFd;
use tokio::io::unix::AsyncFd;
use log::{debug, info};

use crate::api::BluetoothLeRawAdvertisement;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct sockaddr_hci {
    hci_family: libc::sa_family_t,
    hci_dev: u16,
    hci_channel: u16,
}

const BTPROTO_HCI: i32 = 1;
const HCI_CHANNEL_MONITOR: u16 = 3;
const HCI_DEV_NONE: u16 = 0xffff;


pub async fn run_hci_monitor_async(fd: RawFd) -> std::io::Result<()> {
    let async_fd = AsyncFd::new(fd)?;

    loop {
        let mut guard = async_fd.readable().await?;
        match guard.try_io(|| Ok(read_hci_packet(fd))) {
            Ok(Ok(Some(adv))) => {
                debug!("Got advertisement: {:016x} len {}", adv.address, adv.data.len());
                // TODO: forward or enqueue
            }
            Ok(Ok(None)) => {
                debug!("Received non-advertisement or malformed packet");
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

fn read_hci_packet(fd: RawFd) -> Option<BluetoothLeRawAdvertisement> {
    let mut buf = [0u8; 1024];
    let len = unsafe { recv(fd, buf.as_mut_ptr() as *mut c_void, buf.len(), 0) } as usize;
    if len < 7 || buf[0] != 0x03 {
        return None;
    }

    let evt = &buf[6..len];
    if evt.get(0) != Some(&0x3E) {
        return None;
    }

    parse_ble_advertisement(evt)
}

fn parse_ble_advertisement(buf: &[u8]) -> Option<BluetoothLeRawAdvertisement> {
    if buf.len() < 3 || buf[0] != 0x3E {
        return None;
    }

    let plen = buf[1] as usize;
    if buf.len() < 2 + plen {
        return None;
    }

    let subevent = buf[2];
    match subevent {
        0x0D => parse_extended_adv(&buf[3..]),
        0x02 => parse_legacy_adv(&buf[3..]),
        _ => None,
    }
}

fn parse_extended_adv(data: &[u8]) -> Option<BluetoothLeRawAdvertisement> {
    if data.len() < 24 {
        return None;
    }

    let addr = &data[3..9];
    let rssi = data[13] as i8;
    let addr_type = data[2];
    let data_len = data[23] as usize;

    if data.len() < 24 + data_len {
        return None;
    }

    let adv_data = &data[24..24 + data_len];

    info!(
        "EXT ADV: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X} RSSI: {} dBm LEN: {}",
        addr[5], addr[4], addr[3], addr[2], addr[1], addr[0], rssi, adv_data.len()
    );

    Some(BluetoothLeRawAdvertisement {
        address: bdaddr_to_u64(addr),
        rssi: rssi as i32,
        address_type: addr_type as u32,
        data: adv_data.to_vec(),
    })
}

fn parse_legacy_adv(data: &[u8]) -> Option<BluetoothLeRawAdvertisement> {
    if data.len() < 12 {
        return None;
    }

    let addr = &data[3..9];
    let addr_type = data[2];
    let data_len = data[9] as usize;

    if data.len() < 10 + data_len {
        return None;
    }

    let adv_data = &data[10..10 + data_len];
    let rssi = data[10 + data_len] as i8;

    info!(
        "LEG ADV: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X} RSSI: {} dBm LEN: {}",
        addr[5], addr[4], addr[3], addr[2], addr[1], addr[0], rssi, adv_data.len()
    );

    Some(BluetoothLeRawAdvertisement {
        address: bdaddr_to_u64(addr),
        rssi: rssi as i32,
        address_type: addr_type as u32,
        data: adv_data.to_vec(),
    })
}

fn bdaddr_to_u64(addr: &[u8]) -> u64 {
    addr.iter().rev().fold(0u64, |acc, &b| (acc << 8) | b as u64)
}

pub fn open_monitor_socket() -> std::io::Result<RawFd> {
    use libc::{socket, bind, sockaddr, sockaddr_hci, AF_BLUETOOTH, BTPROTO_HCI};
    use std::mem::zeroed;
    use std::mem::size_of;

    let fd = unsafe { socket(AF_BLUETOOTH, libc::SOCK_RAW, BTPROTO_HCI) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }

    let mut addr: sockaddr_hci = unsafe { zeroed() };
    addr.hci_family = AF_BLUETOOTH as u16;
    addr.hci_dev = 0xffff; // HCI_DEV_NONE
    addr.hci_channel = 3;  // HCI_CHANNEL_MONITOR

    let ret = unsafe {
        bind(
            fd,
            &addr as *const _ as *const sockaddr,
            size_of::<sockaddr_hci>() as u32,
        )
    };
    if ret < 0 {
        return Err(std::io::Error::last_os_error());
    }

    Ok(fd)
}
