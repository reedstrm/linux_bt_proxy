import aioesphomeapi
import argparse
import asyncio
import socket
import pprint
import dataclasses

async def main(hostname):
    """Connect to an ESPHome device and get details."""

    # Establish connection
    api = aioesphomeapi.APIClient(hostname, 6053, "")
    await api.connect(login=True)

    pp = pprint.PrettyPrinter(indent=2, width=100)

    # Get API version of the device's firmware
    print("\n=== API Version ===")
    print(api.api_version)

    # Show device details
    print("\n=== Device Info ===")
    device_info = await api.device_info()
    pp.pprint(device_info)

    # List all entities and services of the device
    entities, services = await api.list_entities_services()

    print("\n=== Entities ===")
    for entity in entities:
        cls_name = type(entity).__name__
        print(f"\n--- Entity: {cls_name} ---")
        pp.pprint(entity)

    print("\n=== Services ===")
    for service in services:
        cls_name = type(service).__name__
        print(f"\n--- Service: {cls_name} ---")
        pp.pprint(service)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="ESPHome client tester")
    parser.add_argument("hostname", help="Hostname or IP to connect to")
    args = parser.parse_args()
    asyncio.run(main(args.hostname))
