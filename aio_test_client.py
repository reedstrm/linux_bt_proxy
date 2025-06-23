import aioesphomeapi
import argparse
import asyncio
import socket
import pprint

async def main(hostname):
    """Connect to an ESPHome device and get details."""

    # Establish connection
    api = aioesphomeapi.APIClient(hostname, 6053, "")
    await api.connect(login=True)

    pp = pprint.PrettyPrinter(indent=2, width=120)
    # Get API version of the device's firmware
    pp.pprint(api.api_version)

    # Show device details
    device_info = await api.device_info()
    pp.pprint(device_info)

    # List all entities of the device
    entities = await api.list_entities_services()
    pp.pprint(entities)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="ESPHome client tester")
    parser.add_argument("--hostname", default=socket.gethostname(), help="Hostname or IP to connect to")
    args = parser.parse_args()
    loop = asyncio.get_event_loop()
    loop.run_until_complete(main(args.hostname))
