mod ble;
mod api;

use ble::{open_monitor_socket, run_hci_monitor_async};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let fd = open_monitor_socket()?;
    tokio::spawn(async move {
        if let Err(e) = run_hci_monitor_async(fd).await {
            eprintln!("HCI monitor failed: {e}");
        }
    });

    // Block forever or run other services
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}
