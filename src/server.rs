use bytes::BytesMut;
use log::{debug, info, warn};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;

use crate::api::api::BluetoothLEAdvertisementResponse;
use crate::context::ProxyContext;
use crate::handlers::{
    connect_request, device_info_request, disconnect_request, forward_ble_advertisement,
    hello_request, list_entities_request, ping_request, subscribe_bluetooth_connections_free_request,
};
use crate::proto::next_message;

pub async fn run_tcp_server(
    ctx: Arc<ProxyContext>,
    addr: SocketAddr,
    rx: broadcast::Receiver<BluetoothLEAdvertisementResponse>,
) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!("Listening on {}", addr);

    loop {
        let (stream, peer) = listener.accept().await?;
        info!("New connection from {}", peer);
        let mut client_rx = rx.resubscribe();
        let ctx = Arc::clone(&ctx);
        tokio::spawn(async move {
            if let Err(e) = handle_client(ctx, stream, &mut client_rx).await {
                warn!("Client error: {:?}", e);
            }
        });
    }
}

async fn handle_client(
    ctx: Arc<ProxyContext>,
    mut stream: TcpStream,
    rx: &mut broadcast::Receiver<BluetoothLEAdvertisementResponse>,
) -> std::io::Result<()> {
    let mut buf = BytesMut::with_capacity(1024);

    let mut subscribed = false;
    loop {
        tokio::select! {
            n = stream.read_buf(&mut buf) => {
                match n {
                    Ok(0) => {
                        info!("Client closed connection");
                        break;
                    },
                    Ok(_) => {
                        while let Some((msg_type, payload)) = next_message(&mut buf) {
                            match msg_type {
                                0x01 => hello_request(&mut stream, &payload).await?,
                                0x03 => connect_request(&mut stream, &payload).await?,
                                0x05 => disconnect_request(&mut stream, &payload).await?,
                                0x07 => ping_request(&mut stream, &payload).await?,
                                0x09 => device_info_request(ctx.clone(), &mut stream, &payload).await?,
                                0x0b => list_entities_request(&mut stream, &payload).await?,
                                0x42 => {info!("Handling BLE Adv subscribe request");
                                    subscribed = true},
                                0x50 => subscribe_bluetooth_connections_free_request(&mut stream, &payload).await?,
                                0x57 => {info!("Handling BLE Adv unsubscribe request");
                                    subscribed = false},
                                _ => {
                                    warn!("Unknown message type: 0x{:02x} ({}) from {}", msg_type, msg_type, stream.peer_addr()?.ip());
                                }
                            }
                        }
                    },
                    Err(e) => {
                        warn!("Read Error: {}", e);
                        break;
                    },
                }
            }, // TCP buffer read branch of select!
            ble_msg = rx.recv() => {
                match ble_msg {
                    Ok(advert) => {
                        if subscribed == true {
                            debug!("Forwarding BLE advertisement to {}", stream.peer_addr()?.ip());
                            forward_ble_advertisement(&mut stream, advert).await?;
                        }
                    },
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("Lagged behind on BLE broadcast: {} messages dropped", n);
                    },
                    Err(broadcast::error::RecvError::Closed) => {
                        warn!("BLE broadcast channel closed");
                        break;
                    }
                }
            }, // BLE Advertisement branch of select!
        }
    }
    Ok(())
}
