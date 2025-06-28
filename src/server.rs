use bytes::BytesMut;
use log::{info, warn};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;

use crate::api::api::BluetoothLERawAdvertisement;
use crate::context::ProxyContext;
use crate::handlers::{
    connect_request, device_info_request, hello_request, list_entities_request, ping_request,
};
use crate::proto::next_message;

pub async fn run_tcp_server(
    ctx: Arc<ProxyContext>,
    addr: SocketAddr,
    rx: broadcast::Receiver<BluetoothLERawAdvertisement>,
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
    rx: &mut broadcast::Receiver<BluetoothLERawAdvertisement>,
) -> std::io::Result<()> {
    let mut buf = BytesMut::with_capacity(1024);

    loop {
        let n = stream.read_buf(&mut buf).await?;
        if n == 0 {
            break;
        }

        while let Some((msg_type, payload)) = next_message(&mut buf) {
            match msg_type {
                0x01 => hello_request(&mut stream, &payload).await?,
                0x03 => connect_request(&mut stream, &payload).await?,
                0x07 => ping_request(&mut stream, &payload).await?,
                0x09 => device_info_request(ctx.clone(), &mut stream, &payload).await?,
                0x0b => list_entities_request(&mut stream, &payload).await?,
                _ => {
                    warn!("Unknown message type: 0x{:02x}", msg_type);
                }
            }
        }
    }
    Ok(())
}
