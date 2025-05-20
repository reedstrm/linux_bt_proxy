import socket
import struct
from proto import api_pb2 as api

def main():
    host = "127.0.0.1"  # or use LAN IP directly
    port = 6053

    print(f"Connecting to {host}:{port}...")
    with socket.create_connection((host, port)) as s:
        # Send HelloRequest
        print("Sending HelloRequest")
        s.sendall(b"\x00")  # opcode for HelloRequest

        # Read response header
        header = s.recv(1)
        if header != b"\x01":
            print(f"Unexpected header: {header.hex()}")
            return

        length_data = s.recv(4)
        length = struct.unpack(">I", length_data)[0]

        payload = s.recv(length)
        resp = api.HelloResponse()
        resp.ParseFromString(payload)

        print("✅ Received HelloResponse:")
        print(f"  API Version: {resp.api_version_major}.{resp.api_version_minor}")
        print(f"  Server Info: {resp.server_info or 'N/A'}")

if __name__ == "__main__":
    main()
