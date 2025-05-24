use prost::Message;
use crate::api::BluetoothLeRawAdvertisement;

pub const BLE_ADV_OPCODE: u8 = 0x33;

pub fn serialize_advertisement(msg: &BluetoothLeRawAdvertisement) -> Vec<u8> {
    let mut buf = Vec::new();
    msg.encode(&mut buf).expect("Encoding failed");

    let mut framed = Vec::with_capacity(1 + 2 + buf.len());
    framed.push(BLE_ADV_OPCODE);
    let len_prefix = encode_varint(buf.len() as u64);
    framed.extend_from_slice(&len_prefix);
    framed.extend_from_slice(&buf);
    framed
}


pub fn encode_varint(mut value: u64) -> Vec<u8> {
    let mut buf = Vec::new();
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if value == 0 {
            break;
        }
    }
    buf
}
