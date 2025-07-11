use futures_util::stream::StreamExt;
use std::collections::HashMap;
use log::{debug, info, warn};

use tokio::sync::broadcast::Sender;

use zbus::fdo::PropertiesProxy;
use zbus::match_rule::MatchRule;
use zbus::names::InterfaceName;
use zbus::zvariant::{ObjectPath, OwnedValue, Dict};
use zbus::{message::Type, Connection, MessageStream, Proxy};


use crate::api::api::{BluetoothLEAdvertisementResponse, BluetoothServiceData};

pub async fn run_bluez_advertisement_listener(
    adapter_index: u16,
    tx: Sender<BluetoothLEAdvertisementResponse>,
) -> zbus::Result<()> {
    let conn = Connection::system().await?;
    let adapter_rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .interface("org.freedesktop.DBus.Properties")?
        .member("PropertiesChanged")?
        .arg(0, "org.bluez.Adapter1")?
        .build();

    let props_rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .interface("org.freedesktop.DBus.Properties")?
        .member("PropertiesChanged")?
        .arg(0, "org.bluez.Device1")?
        .build();

    let iface_rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .interface("org.freedesktop.DBus.ObjectManager")?
        .member("InterfacesAdded")?
        .build();

    let mut adapter_stream = MessageStream::for_match_rule(adapter_rule, &conn, None).await?;
    let mut props_stream = MessageStream::for_match_rule(props_rule, &conn, None).await?;
    let mut iface_stream = MessageStream::for_match_rule(iface_rule, &conn, None).await?;

    // All streams ready: now start discovery
    try_start_discovery(&conn, adapter_index).await?;

    loop {
        tokio::select! {
            maybe_msg = adapter_stream.next() => {
                if let Some(Ok(msg)) = maybe_msg {
                    let body = msg.body();
                    let (interface, changed, _invalidated): (String, HashMap<String, OwnedValue>, Vec<String>) =
                        body.deserialize()?;

                    if interface == "org.bluez.Adapter1" {
                        if let Some(value) = changed.get("Discovering") {
                            if let Some(is_discovering) = value.downcast_ref::<bool>().ok() {
                                if !is_discovering {
                                    info!("Discovery was turned off — restarting discovery.");
                                    try_start_discovery(&conn, adapter_index).await?;
                                }
                            }
                        }
                    }
                }
            }

            maybe_msg = props_stream.next() => {
                if let Some(msg) = maybe_msg.transpose()? {
                    let device_path = msg.header().path().map(|p| p.to_string());
                    if let Some(path) = device_path {
                        match get_device_properties(&conn, &path).await {
                            Ok(props) => {
                                debug!("Changed properties for device {}:", path);
                                match build_advertisement_response(&props) {
                                    Some(msg) => {
                                        if let Err(e) = tx.send(msg) {
                                            warn!("Failed to send advertisement response: {}", e);
                                        }
                                    }
                                    None => {
                                        warn!("Failed to build advertisement response for {}", path);
                                    }
                                };
                                }
                            Err(e) => {
                                warn!("Failed to fetch properties for {}: {}", path, e);
                            }
                        }
                    }
                }
            }

            maybe_msg = iface_stream.next() => {
                if let Some(msg) = maybe_msg.transpose()? {
                    let body = msg.body();
                    let (path, interfaces): (
                        ObjectPath<'_>,
                        HashMap<String, HashMap<String, OwnedValue>>
                    ) = body.deserialize()?;

                    debug!("InterfacesAdded at path: {}", path);
                    match interfaces.get("org.bluez.Device1") {
                        Some(props) => {
                            debug!("New properties for device {}:", path);
                            match build_advertisement_response(&props) {
                                Some(msg) => {
                                    if let Err(e) = tx.send(msg) {
                                        warn!("Failed to send advertisement response: {}", e);
                                    }
                                }
                                None => {
                                    warn!("Failed to build advertisement response for {}", path);
                                }
                            };
                        }
                        _ => {
                            debug!("Failed to fetch properties for {}", path);
                        }
                    }
                }
            }
        } // select
    } // loop

    Ok(())
}

fn build_advertisement_response(
    props: &HashMap<String, OwnedValue>,
) -> Option<BluetoothLEAdvertisementResponse> {
    let mac_opt = props
        .get("Address")
        .and_then(|v| v.downcast_ref::<String>().ok());

    let address_type = props
        .get("AddressType")
        .and_then(|v| v.downcast_ref::<String>().ok())
        .map(|s| if s == "random" { 1 } else { 0 })
        .unwrap_or(0);

    let rssi = props
        .get("RSSI")
        .and_then(|v| v.downcast_ref::<i32>().ok())
        .unwrap_or(-127);

    let name = props
        .get("Name")
        .and_then(|v| v.downcast_ref::<String>().ok())
        .map_or_else(Vec::new, |s| s.into_bytes());

    let service_uuids = props
        .get("UUIDs")
        .cloned()
        .and_then(|v| Vec::<String>::try_from(v).ok())
        .unwrap_or_default();

    let service_data = extract_service_data(props.get("ServiceData"), true)
        .unwrap_or_default();

    let manufacturer_data = extract_service_data(props.get("ManufacturerData"), false)
        .unwrap_or_default();

    if let Some(mac_str) = mac_opt {
        let msg = BluetoothLEAdvertisementResponse {
            address: parse_ble_address(&mac_str),
            address_type,
            name,
            rssi,
            service_uuids,
            service_data,
            manufacturer_data,
            ..Default::default()
        };
        return Some(msg);
    }
    None
}

async fn get_device_properties(
    conn: &Connection,
    path: &str,
) -> zbus::Result<HashMap<String, OwnedValue>> {
    let proxy = PropertiesProxy::new(
        conn,
        "org.bluez",
        ObjectPath::try_from(path)?,
    )
    .await?;

    let props: HashMap<String, OwnedValue> = proxy.get_all(InterfaceName::try_from("org.bluez.Device1")?).await?;

    Ok(props)
}

//fn print_props(props: &HashMap<String, OwnedValue>) {
//    for (key, value) in props {
//        println!("  {} => {:?}", key, value);
//    }
//}

async fn try_start_discovery(conn: &Connection, adapter_index: u16) -> zbus::Result<()> {
    let adapter_path = format!("/org/bluez/hci{}", adapter_index);
    let proxy = Proxy::new(
        conn,
        "org.bluez",
        ObjectPath::try_from(adapter_path.as_str())?,
        "org.bluez.Adapter1",
    )
    .await?;

    proxy
        .call_method("SetDiscoveryFilter", &(HashMap::<&str, OwnedValue>::new()))
        .await?;
    match proxy.call_method("StartDiscovery", &()).await {
        Ok(_) => info!("Discovery started"),
        Err(zbus::Error::MethodError(ref name, _, _))
            if name.as_str() == "org.bluez.Error.InProgress" =>
        {
            warn!("Discovery already in progress");
        }
        Err(e) => return Err(e),
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
