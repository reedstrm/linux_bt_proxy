import asyncio
import logging
import socket
import struct
import argparse
from zeroconf.asyncio import AsyncZeroconf
from zeroconf import ServiceInfo
from bleak import BleakScanner, AdvertisementData
import time

# Load compiled protobuf messages
from proto import api_pb2 as api

# === Logging Setup ===
logging.basicConfig(level=logging.INFO)
log = logging.getLogger("bt-proxy")

# === mDNS Advertisement ===
async def advertise_mdns(hostname, ip_address, port, mac):
    service_name = f"{hostname}._esphomelib._tcp.local."
    info = ServiceInfo(
        type_="_esphomelib._tcp.local.",
        name=service_name,
        addresses=[socket.inet_aton(ip_address)],
        port=port,
        properties={"mac": mac.replace(":", "").lower()},
        server=f"{hostname}.local.",
    )
    azc = AsyncZeroconf()
    await azc.async_register_service(info)
    log.info(f"mDNS advertised as {service_name} at {ip_address}:{port}")

# === ESPHome Protocol ===
clients = set()

async def send_advertisement_to_clients(device, adv_data: AdvertisementData):
    msg = api.BluetoothLERawAdvertisement()
    msg.address = device.address.replace(":", "")
    msg.rssi = device.rssi
    msg.name = device.name or ""
    msg.timestamp = int(time.time() * 1000)
    msg.data = adv_data.bytes or b""

    data = msg.SerializeToString()
    packet = b'\x33' + struct.pack('>I', len(data)) + data  # opcode 0x33 = BluetoothLERawAdvertisement

    for writer in clients:
        try:
            writer.write(packet)
            await writer.drain()
        except Exception as e:
            log.warning(f"Failed to send to client: {e}")

async def ble_scan_loop():
    def detection_callback(device, adv_data):
        return asyncio.create_task(send_advertisement_to_clients(device, adv_data))

    scanner = BleakScanner(detection_callback)
    while True:
        try:
            await scanner.start()
            await asyncio.sleep(5)
            await scanner.stop()
        except Exception as e:
            log.error(f"BLE scan error: {e}")
            await asyncio.sleep(5)

async def handle_client(reader, writer):
    addr = writer.get_extra_info('peername')
    log.info(f"Accepted connection from {addr}")
    clients.add(writer)

    try:
        while True:
            header = await reader.readexactly(1)
            if header == b'\x00':
                # HelloRequest: send HelloResponse
                payload = api.HelloResponse()
                payload.api_version_major = 1
                payload.api_version_minor = 6
                data = payload.SerializeToString()
                writer.write(b'\x01' + struct.pack('>I', len(data)) + data)
                await writer.drain()
                log.info("Sent HelloResponse")
            else:
                log.warning(f"Unknown header byte: {header.hex()}")
                break
    except asyncio.IncompleteReadError:
        log.info(f"Client disconnected: {addr}")
    finally:
        clients.discard(writer)
        writer.close()
        await writer.wait_closed()

async def start_server(port):
    server = await asyncio.start_server(handle_client, "0.0.0.0", port)
    log.info(f"TCP server started on port {port}")
    async with server:
        await server.serve_forever()

async def main(args):
    IP_ADDRESS = get_lan_ip()
    await advertise_mdns(args.hostname, IP_ADDRESS, args.port, args.mac)
    await asyncio.gather(
        start_server(args.port),
        ble_scan_loop()
    )

# === Entry Point ===
if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="ESPHome-compatible Bluetooth Proxy")
    parser.add_argument("--hostname", default=socket.gethostname(), help="Hostname to advertise")
    parser.add_argument("--mac", default="AA:BB:CC:DD:EE:FF", help="Fake MAC address to advertise")
    parser.add_argument("--port", type=int, default=6053, help="Port to listen on")
    args = parser.parse_args()

    def get_lan_ip():
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        try:
            s.connect(("8.8.8.8", 80))
            return s.getsockname()[0]
        finally:
            s.close()

    try:
        asyncio.run(main(args))
    except KeyboardInterrupt:
        log.info("Shutting down proxy.")
