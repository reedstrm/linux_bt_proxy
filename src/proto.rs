use prost::Message;
use crate::api::BluetoothLeRawAdvertisement;

pub const BLE_ADV_OPCODE: u8 = 0x33;

pub fn serialize_advertisement(msg: &BluetoothLeRawAdvertisement) -> Vec<u8> {
    let mut buf = Vec::new();
    msg.encode(&mut buf).expect("Encoding failed");

    let mut framed = Vec::with_capacity(1 + 2 + buf.len());
    framed.push(BLE_ADV_OPCODE);
    let len = buf.len() as u16;
    framed.extend_from_slice(&len.to_le_bytes());
    framed.extend_from_slice(&buf);
    framed
}
