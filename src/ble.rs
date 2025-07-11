use std::collections::HashMap;
use log::{debug, info};

use futures_util::StreamExt;
use tokio::sync::broadcast::Sender;
use zbus::{
    zvariant::{ObjectPath, OwnedValue,  Dict},
    zbus_core::MessageType, 
    Connection, MatchRule, Message, MessageStream, Proxy,
};

use crate::api::api::{BluetoothLEAdvertisementResponse, BluetoothServiceData};

pub async fn run_bluez_advertisement_listener(
    adapter_index: u16,
    tx: Sender<BluetoothLEAdvertisementResponse>,
) -> zbus::Result<()> {
    let conn = Connection::system().await?;

    let adapter_path = format!("/org/bluez/hci{}", adapter_index);
    let adapter_proxy = Proxy::new(
        &conn,
        "org.bluez",
        ObjectPath::try_from(adapter_path.as_str())?,
        "org.bluez.Adapter1",
    )
    .await?;

    adapter_proxy
        .call_method("SetDiscoveryFilter", &(HashMap::<&str, OwnedValue>::new()))
        .await?;
    adapter_proxy.call_method("StartDiscovery", &()).await?;

    let mut stream = MessageStream::from(conn);

    let match_rule = MatchRule::builder()
        .member("PropertiesChanged")?
        .msg_type(MessageType::Signal)
        .interface("org.freedesktop.DBus.Properties")?
        .build();

    while let Some(Ok(msg)) = stream.next().await {
        if match_rule.matches(&msg)? {

            info!("Raw D-Bus msg: {:?}", msg);

            let Ok((interface, changed_props, _)) = msg.body().deserialize::<(String, HashMap<String, OwnedValue>, Vec<String>)>() else {
                continue;
            };

            if interface != "org.bluez.Device1" {
                continue;
            }

            let mac_opt = changed_props
                .get("Address")
                .and_then(|v| v.downcast_ref::<String>().ok());

            let address_type = changed_props
                .get("AddressType")
                .and_then(|v| v.downcast_ref::<String>().ok())
                .map(|s| if s == "random" { 1 } else { 0 })
                .unwrap_or(0);

            let rssi = changed_props
                .get("RSSI")
                .and_then(|v| v.downcast_ref::<i32>().ok());

            let name = changed_props
                .get("Name")
                .and_then(|v| v.downcast_ref::<String>().ok());

            let service_uuids = changed_props
                .get("UUIDs")
                .cloned()
                .and_then(|v| Vec::<String>::try_from(v).ok())
                .unwrap_or_default();

            let service_data = extract_service_data(changed_props.get("ServiceData"), true)
                .unwrap_or_default();

            let manufacturer_data = extract_service_data(changed_props.get("ManufacturerData"), false)
                .unwrap_or_default();

            if let Some(mac_str) = mac_opt {
                let msg = BluetoothLEAdvertisementResponse {
                    address: parse_ble_address(&mac_str),
                    address_type,
                    name: name.map_or_else(Vec::new, |s| s.into_bytes()),
                    rssi: rssi.unwrap_or(-127),
                    service_uuids,
                    service_data,
                    manufacturer_data,
                    ..Default::default()
                };

                let _ = tx.send(msg);
            }
        }
    }

    Ok(())
}

fn parse_ble_address(address: &str) -> u64 {
    address
        .split(':')
        .fold(0, |acc, part| (acc << 8) | u8::from_str_radix(part, 16).unwrap_or(0) as u64)
}

fn extract_service_data(
    value_opt: Option<&OwnedValue>,
    is_service: bool,
) -> Result<Vec<BluetoothServiceData>, Box<dyn std::error::Error>> {
    // If no value present, return empty list
    let Some(v) = value_opt else {
        return Ok(Vec::new());
    };

    let dict = Dict::try_from(v.to_owned())?;

    let mut entries = Vec::new();

    for (k, v) in dict {
        // Extract UUID
        let uuid = if is_service {
            k.downcast::<String>().ok()
        } else {
            k.downcast::<u16>().ok().map(|id| id.to_string())
        };

        let data: Result<Vec<u8>, _> = v.downcast();

        // If both present, create entry
        if let (Some(uuid), Ok(data)) = (uuid, data) {
            entries.push(BluetoothServiceData {
                uuid,
                data,
                ..Default::default()
            });
        }
    }

    Ok(entries)
}
