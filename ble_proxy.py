import asyncio
import logging
import socket
import struct
from zeroconf import Zeroconf, ServiceInfo

# Load compiled protobuf messages
from proto import api_pb2 as api

# === Configuration ===
SERVICE_NAME = "linux-bt-proxy._esphomelib._tcp.local."
HOSTNAME = socket.gethostname()
PORT = 6053
IP_ADDRESS = socket.gethostbyname(HOSTNAME)
MAC = "AA:BB:CC:DD:EE:FF"  # Use a fixed or actual MAC if you have it

# === Logging Setup ===
logging.basicConfig(level=logging.INFO)
log = logging.getLogger("bt-proxy")

# === mDNS Advertisement ===
def advertise_mdns():
    zeroconf = Zeroconf()
    info = ServiceInfo(
        type_="_esphomelib._tcp.local.",
        name=SERVICE_NAME,
        addresses=[socket.inet_aton(IP_ADDRESS)],
        port=PORT,
        properties={"mac": MAC.replace(":", "").lower()},
        server=f"{HOSTNAME}.local.",
    )
    zeroconf.register_service(info)
    log.info(f"mDNS advertised as {SERVICE_NAME} at {IP_ADDRESS}:{PORT}")

# === ESPHome Protocol ===
async def handle_client(reader, writer):
    addr = writer.get_extra_info('peername')
    log.info(f"Accepted connection from {addr}")

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
        writer.close()
        await writer.wait_closed()

async def start_server():
    server = await asyncio.start_server(handle_client, "0.0.0.0", PORT)
    log.info(f"TCP server started on port {PORT}")
    async with server:
        await server.serve_forever()

# === Entry Point ===
if __name__ == "__main__":
    advertise_mdns()
    try:
        asyncio.run(start_server())
    except KeyboardInterrupt:
        log.info("Shutting down proxy.")
