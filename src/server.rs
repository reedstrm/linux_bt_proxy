use tokio::net::{TcpListener, TcpStream};
use tokio::io::AsyncWriteExt;
use tokio::sync::broadcast;
use std::net::SocketAddr;

use crate::proto::serialize_advertisement;
use crate::api::BluetoothLeRawAdvertisement;

pub async fn run_tcp_server(addr: SocketAddr, rx: broadcast::Receiver<BluetoothLeRawAdvertisement>) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    log::info!("Listening on {}", addr);

    loop {
        let (stream, _) = listener.accept().await?;
        let mut client_rx = rx.resubscribe();
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream, &mut client_rx).await {
                log::warn!("Client error: {:?}", e);
            }
        });
    }
}

async fn handle_client(mut stream: TcpStream, rx: &mut broadcast::Receiver<BluetoothLeRawAdvertisement>) -> std::io::Result<()> {
    while let Ok(msg) = rx.recv().await {
        let data = serialize_advertisement(&msg);
        stream.write_all(&data).await?;
        stream.flush().await?;
    }
    Ok(())
}
