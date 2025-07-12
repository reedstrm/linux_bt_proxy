use crate::api::api_options;
use bytes::{Bytes, BytesMut};
use protobuf::MessageFull;
use std::io::{Error, ErrorKind};

pub fn get_message_id<M: MessageFull>() -> u8 {
    protobuf::ext::ExtFieldOptional::get(
        &api_options::exts::id,
        M::descriptor().proto().options.as_ref().unwrap(),
    )
    .expect("Missing extension id") as u8
}

/// Decodes a protobuf varint from a byte slice, returning the value and the number of bytes consumed.
pub fn decode_varint(buf: &[u8]) -> Result<(u64, usize), Error> {
    let mut result = 0u64;
    let mut shift = 0;
    for (i, &byte) in buf.iter().enumerate() {
        let val = (byte & 0x7F) as u64;
        result |= val << shift;
        if byte & 0x80 == 0 {
            return Ok((result, i + 1));
        }
        shift += 7;
        if shift > 63 {
            return Err(Error::new(ErrorKind::InvalidData, "Varint too long"));
        }
    }
    Err(Error::new(ErrorKind::UnexpectedEof, "Incomplete varint"))
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

pub fn next_message(buf: &mut BytesMut) -> Option<(u32, Bytes)> {
    // Step 1: check framing byte
    if buf.is_empty() || buf[0] != 0x00 {
        return None;
    }

    let mut offset = 1;

    // Step 2: parse total message length
    let (length, len_size) = decode_varint(&buf[offset..]).ok()?;
    offset += len_size;

    // Step 3: make sure buffer contains entire message
    if buf.len() < offset + length as usize {
        return None;
    }

    // Step 4: parse message type
    let (msg_type, type_size) = decode_varint(&buf[offset..]).ok()?;
    offset += type_size;

    let payload_len = length as usize;
    let mut head = buf.split_to(offset + payload_len); // remove the message from buf
    let payload = head.split_off(offset).freeze(); // skip to payload and freeze it

    Some((msg_type as u32, payload))
}
