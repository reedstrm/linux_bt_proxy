use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::broadcast;
use std::net::SocketAddr;
use prost::Message;
use log::{warn,info,debug};

use crate::proto::{serialize_advertisement};
use crate::api::{BluetoothLeRawAdvertisement, HelloRequest, HelloResponse};

/// Decodes a protobuf varint from a byte slice, returning the value and the number of bytes consumed.
/// Returns None if the input does not contain a valid varint.
pub fn decode_varint(buf: &[u8]) -> Option<(u64, usize)> {
    let mut result = 0u64;
    let mut shift = 0;
    for (i, &byte) in buf.iter().enumerate() {
        let val = (byte & 0x7F) as u64;
        result |= val << shift;
        if byte & 0x80 == 0 {
            return Some((result, i + 1));
        }
        shift += 7;
        if shift > 63 {
            // varint is too long
            return None;
        }
    }
    None
}

pub async fn run_tcp_server(addr: SocketAddr, rx: broadcast::Receiver<BluetoothLeRawAdvertisement>) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!("Listening on {}", addr);

    loop {
        let (stream, peer) = listener.accept().await?;
        info!("New connection from {}", peer);
        let mut client_rx = rx.resubscribe();
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream, &mut client_rx).await {
                warn!("Client error: {:?}", e);
            }
        });
    }
}

async fn handle_client(mut stream: TcpStream, rx: &mut broadcast::Receiver<BluetoothLeRawAdvertisement>) -> std::io::Result<()> {
    let mut buf = [0u8; 1024];

    loop {
        let n = stream.read(&mut buf).await?;
        if n == 0 {
            break;
        }

        debug!("Received message: {:02x?}", &buf[..n]);

        let opcode = buf[0];
        if let Some((msg_len, varint_size)) = decode_varint(&buf[1..]) {
            let total_size = 1 + varint_size + msg_len as usize;
            if buf.len() < total_size {
                warn!("Incomplete message: expected {}, got {}", total_size, buf.len());
                return Ok(());
            }

            let msg_buf = &buf[1 + varint_size .. total_size];
            match opcode {
                0x00 => {  // HelloRequest
                    info!("Handling HelloRequest");
                    let _req = HelloRequest::decode(msg_buf)?;
                    let resp = HelloResponse {
                        api_version_major: 1,
                        api_version_minor: 7,
                        server_info: "linux_bt_proxy".into(),
                        name: "Linux Bluetooth Proxy".into(),
                    };
                    let mut out = Vec::new();
                    resp.encode(&mut out).expect("Failed to encode HelloResponse");
                    let mut framed = vec![0x01]; // HelloResponse opcode
                    framed.extend_from_slice(&(out.len() as u16).to_le_bytes());
                    framed.extend_from_slice(&out);
                    stream.write_all(&framed).await?;

                    // Start pushing advertisements
                    while let Ok(msg) = rx.recv().await {
                    let data = serialize_advertisement(&msg);
                    stream.write_all(&data).await?;
                    }
                },
                _ => {
                    warn!("Unknown opcode: 0x{:02x}", opcode);
                }
            }
        }
    }
    Ok(())
}
