import argparse
import socket
import struct
from proto import api_pb2 as api

def read_varint(sock):
    """Reads a varint from a socket and returns (length, bytes_consumed)."""
    result = 0
    shift = 0
    bytes_read = 0

    while True:
        byte = sock.recv(1)
        if not byte:
            raise ConnectionError("Socket closed before varint could be read")

        byte_val = byte[0]
        result |= (byte_val & 0x7F) << shift
        bytes_read += 1

        if not (byte_val & 0x80):
            break
        shift += 7

        if shift >= 64:
            raise ValueError("Varint too long")

    return result

def encode_varint(value):
    """Encode an integer as a protobuf varint."""
    buf = []
    while True:
        to_write = value & 0x7F
        value >>= 7
        if value:
            buf.append(to_write | 0x80)
        else:
            buf.append(to_write)
            break
    return bytes(buf)


def send_hello_request(sock):
    hello = api.HelloRequest()
    hello.client_info='Test client'
    hello.api_version_major = 1
    hello.api_version_minor = 10
    payload = hello.SerializeToString()
    sock.sendall(b'\x00' + encode_varint(len(payload)) + b'\x01' + payload)

def receive_message(sock):
    preamble = read_varint(sock)
    if preamble != 0x00:
       print('Bad preamble')
       exit(1)
    length = read_varint(sock)
    opcode = read_varint(sock)
    payload = sock.recv(length)
    return opcode, payload

def main(host):
    port = 6053

    print(f"Connecting to {host}:{port}...")
    with socket.create_connection((host, port)) as sock:
        print("Sending HelloRequest")
        send_hello_request(sock)

        print("Receiving HelloResponse")
        opcode, payload  = receive_message(sock)
        print(opcode, payload)
        print(type(opcode))
        if payload and opcode == 0x02:
            hello_resp = api.HelloResponse()
            hello_resp.ParseFromString(payload)
            print(f"Received HelloResponse: server_info={hello_resp.server_info}, name={hello_resp.name}")

        print("Listening for BLE advertisements...")
        try:
            while True:
                data = receive_message(sock)
                if data is None:
                    break
                opcode, payload = data
                if opcode == 0x33:
                    adv = api.BluetoothLERawAdvertisement()
                    adv.ParseFromString(payload)
                    print(f"BLE ADV from {adv.address:012X} RSSI={adv.rssi} len={len(adv.data)}")
                else:
                    print(f"Unexpected opcode: {opcode}")
        except KeyboardInterrupt:
            print("Interrupted by user")
if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="ESPHome client tester")
    parser.add_argument("--hostname", default=socket.gethostname(), help="Hostname or IP to connect to")
    args = parser.parse_args()
    main(args.hostname)
