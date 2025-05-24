use libc::{self, c_int, c_uint, c_ushort, c_ulong, c_void};
use std::mem::zeroed;

pub fn get_bt_mac(hci_index: u16) -> Option<String> {
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

    log::debug!("Attempting to open HCI device hci{}", hci_index);

    let sock = unsafe { libc::socket(AF_BLUETOOTH, SOCK_RAW, BTPROTO_HCI) };
    if sock < 0 {
        log::error!("Failed to open raw HCI socket: errno {}", sock);
        return None;
    }

    let mut devinfo: HciDevInfo = unsafe { zeroed() };
    devinfo.dev_id = hci_index;

    let ret = unsafe {
        libc::ioctl(sock, HCIGETDEVINFO, &mut devinfo as *mut _ as *mut c_void)
    };

    unsafe {
        libc::close(sock);
    }

    if ret < 0 {
        log::error!("ioctl HCIGETDEVINFO failed for hci{} (ret = {})", hci_index, ret);
        return None;
    }

    let mac = format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        devinfo.bdaddr[5],
        devinfo.bdaddr[4],
        devinfo.bdaddr[3],
        devinfo.bdaddr[2],
        devinfo.bdaddr[1],
        devinfo.bdaddr[0],
    );

    log::info!("Retrieved MAC for hci{}: {}", hci_index, mac);
    Some(mac)
}
