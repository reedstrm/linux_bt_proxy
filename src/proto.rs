use crate::api::api::BluetoothLERawAdvertisement;
use protobuf::Message;

const adv_opcode = BluetoothLERawAdvertisement::descriptor()
    .options()
    .get_extension(api_options::id);


pub fn serialize_advertisement(msg: &BluetoothLERawAdvertisement) -> Vec<u8> {
    let mut buf = msg.write_to_vec().expect("Encoding failed");

    let mut framed = Vec::with_capacity(1 + 2 + buf.len());
    framed.push(adv_opcode);
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
