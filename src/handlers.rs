use protobuf::Message;
//use protobuf::{EnumOrUnknown, Message};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use crate::api::api::{
    BluetoothConnectionsFreeResponse, //,
                                      //  SensorStateClass, ListEntitiesSensorResponse
    BluetoothLEAdvertisementResponse,
    ConnectRequest,
    ConnectResponse,
    DeviceInfoRequest,
    DeviceInfoResponse,
    DisconnectRequest,
    DisconnectResponse,
    HelloRequest,
    HelloResponse,
    ListEntitiesDoneResponse,
    ListEntitiesRequest,
    PingRequest,
    PingResponse,
    SubscribeBluetoothConnectionsFreeRequest,
};
use crate::context::ProxyContext;
use crate::proto::{encode_varint, get_message_id};
use crate::utils::format_mac;
use log::info;

fn encode_response<M: Message>(msg_type: u32, message: &M) -> Result<Vec<u8>, std::io::Error> {
    let size = message.compute_size() as usize;

    let mut out = Vec::with_capacity(1 + 5 + 5 + size); // start + varints + payload
    out.push(0x00); // Plaintext framing

    out.extend_from_slice(&encode_varint(size as u64));
    out.extend_from_slice(&encode_varint(msg_type as u64));

    message.write_to_writer(&mut out)?; // ✨ direct append

    Ok(out)
}

pub async fn hello_request(stream: &mut TcpStream, payload: &[u8]) -> Result<(), std::io::Error> {
    // HelloRequest -> inital contact from HA server
    info!("Handling HelloRequest from {}", stream.peer_addr()?.ip());
    let _req = HelloRequest::parse_from_bytes(payload)?;
    let resp = HelloResponse {
        api_version_major: 1,
        api_version_minor: 10,
        server_info: "linux_bt_proxy".into(),
        name: "Linux Bluetooth Proxy".into(),
        ..Default::default() // fill special_fields
    };

    let hello_resp_type = get_message_id::<HelloResponse>();
    stream
        .write_all(&encode_response(hello_resp_type as u32, &resp)?)
        .await?;
    Ok(())
}

pub async fn connect_request(stream: &mut TcpStream, payload: &[u8]) -> Result<(), std::io::Error> {
    // ConnectRequest -> no pasword plaintext, reply with empty Response
    info!("Handling ConnectRequest from {}", stream.peer_addr()?.ip());
    let _req = ConnectRequest::parse_from_bytes(payload)?;
    let resp = ConnectResponse::new();
    let connect_resp_type = get_message_id::<ConnectResponse>();
    stream
        .write_all(&encode_response(connect_resp_type as u32, &resp)?)
        .await?;
    Ok(())
}

pub async fn disconnect_request(
    stream: &mut TcpStream,
    payload: &[u8],
) -> Result<(), std::io::Error> {
    // DisconnectRequest
    info!(
        "Handling DisconnectRequest from {}",
        stream.peer_addr()?.ip()
    );
    let _req = DisconnectRequest::parse_from_bytes(payload)?;
    let resp = DisconnectResponse::new();
    let disconnect_resp_type = get_message_id::<DisconnectResponse>();
    stream
        .write_all(&encode_response(disconnect_resp_type as u32, &resp)?)
        .await?;
    stream.shutdown().await?;
    Ok(())
}

pub async fn ping_request(stream: &mut TcpStream, payload: &[u8]) -> Result<(), std::io::Error> {
    // Ping -> reply with pong (PingResponse, actually)
    info!("Handling PingRequest from {}", stream.peer_addr()?.ip());
    let _req = PingRequest::parse_from_bytes(payload)?;
    let resp = PingResponse::new();
    let ping_resp_type = get_message_id::<PingResponse>();
    stream
        .write_all(&encode_response(ping_resp_type as u32, &resp)?)
        .await?;
    Ok(())
}

pub async fn subscribe_bluetooth_connections_free_request(
    stream: &mut TcpStream,
    payload: &[u8],
) -> Result<(), std::io::Error> {
    // Bluetooth Connections Free -> BluetoothConnectionsFreeResponse
    info!(
        "Handling BluetoothConnectionsFree from {}",
        stream.peer_addr()?.ip()
    );
    let _req = SubscribeBluetoothConnectionsFreeRequest::parse_from_bytes(payload)?;
    let resp = BluetoothConnectionsFreeResponse {
        free: 0,
        limit: 0,
        ..Default::default()
    };
    let resp_type = get_message_id::<BluetoothConnectionsFreeResponse>();
    stream
        .write_all(&encode_response(resp_type as u32, &resp)?)
        .await?;
    Ok(())
}

pub async fn device_info_request(
    ctx: Arc<ProxyContext>,
    stream: &mut TcpStream,
    payload: &[u8],
) -> Result<(), std::io::Error> {
    // DeviceInfoRequest -> reply with values from ProxyContext
    info!(
        "Handling DeviceInfoRequest from {}",
        stream.peer_addr()?.ip()
    );
    let _req = DeviceInfoRequest::parse_from_bytes(payload)?;

    let resp = DeviceInfoResponse {
        name: format!("Linux BT Proxy: {}", ctx.hostname),
        mac_address: format_mac(&ctx.net_mac, ":"),

        // A string describing the ESPHome version. For example "1.10.0"
        // FIXME: I'd send my own version, but am sending the version I'm
        // mimicking, since I had some issues getting HomeAssistant to register
        // the proxy with my own version.
        esphome_version: "2024.8.3".to_string(),

        // A string describing the date of compilation, this is generated by the compiler
        // and therefore may not be in the same format all the time.
        // If the user isn't using ESPHome, this will also not be set.
        compilation_time: ctx.build_time.to_string(),

        // The model of the board. For example NodeMCU
        model: "Linux".to_string(),

        // The esphome project details if set
        // FIXME: if I set these, homeassistant fails to register the proxy
        // project_name: "linux_bt_proxy".to_string(),
        // project_version: ctx.version.to_string(),
        legacy_bluetooth_proxy_version: 5,
        bluetooth_proxy_feature_flags: 0x08 | 0x10 | 0x20, // 0x38

        friendly_name: format!("Linux BT Proxy: {}", ctx.hostname),

        // The Bluetooth mac address of the device. For example "AC:BC:32:89:0E:AA"
        bluetooth_mac_address: format_mac(&ctx.bt_mac, ":"),

        // Supports receiving and saving api encryption key
        api_encryption_supported: false,
        ..Default::default()
    };
    let device_info_resp_type = get_message_id::<DeviceInfoResponse>();
    stream
        .write_all(&encode_response(device_info_resp_type as u32, &resp)?)
        .await?;
    Ok(())
}

pub async fn list_entities_request(
    stream: &mut TcpStream,
    payload: &[u8],
) -> Result<(), std::io::Error> {
    // ListEntitiesRequest
    info!(
        "Handling ListEntitiesRequest from {}",
        stream.peer_addr()?.ip()
    );
    let _req = ListEntitiesRequest::parse_from_bytes(payload)?;
    //    let resp = ListEntitiesSensorResponse {
    //        object_id: "uptime".to_string(),
    //        unique_id: "uptime_sensor".to_string(),
    //        name: "Proxy Service Uptime".to_string(),
    //        unit_of_measurement: "s".to_string(),
    //        accuracy_decimals: 0,
    //        device_class: "duration".to_string(),
    //        state_class: EnumOrUnknown::new(SensorStateClass::STATE_CLASS_TOTAL_INCREASING),
    //        icon: "mdi:clock-time-four-outline".to_string(),
    //        key: 42 as u32,
    //        ..Default::default()
    //    };
    //    let list_entitities_sensor_resp_type = get_message_id::<ListEntitiesSensorResponse>();
    //    stream
    //        .write_all(&encode_response(list_entitities_sensor_resp_type as u32, &resp)?)
    //        .await?;
    let resp = ListEntitiesDoneResponse::new();
    let list_entitities_done_resp_type = get_message_id::<ListEntitiesDoneResponse>();
    stream
        .write_all(&encode_response(
            list_entitities_done_resp_type as u32,
            &resp,
        )?)
        .await?;
    Ok(())
}

pub async fn forward_ble_advertisement(
    stream: &mut TcpStream,
    adv: BluetoothLEAdvertisementResponse,
) -> Result<(), std::io::Error> {
    let ble_adv_res_type = get_message_id::<BluetoothLEAdvertisementResponse>();
    stream
        .write_all(&encode_response(ble_adv_res_type as u32, &adv)?)
        .await?;
    Ok(())
}
