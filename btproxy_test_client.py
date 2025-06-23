import aioesphomeapi
import argparse
import asyncio
import pprint
import contextlib
from aioesphomeapi.api_pb2 import BluetoothLEAdvertisementResponse
from aioesphomeapi.core import InvalidAuthAPIError, APIConnectionError

pp = pprint.PrettyPrinter(indent=2, width=100)

async def main(hostname, password):
    api = aioesphomeapi.APIClient(hostname, 6053, password)

    def handle_ble_adv(msg: BluetoothLEAdvertisementResponse):
        pp.pprint(msg)

    try:
        await api.connect(login=True)
    except InvalidAuthAPIError:
        print("ERROR: Invalid password!")
        return
    except APIConnectionError as e:
        print(f"ERROR: Failed to connect to {hostname}: {e}")
        return

    print(f"Connected to {hostname}")
    print("API Version:", api.api_version)
    print("Listening for BLE advertisements...\n")

    unsubscribe = None
    try:
        unsubscribe = api.subscribe_bluetooth_le_advertisements(handle_ble_adv)
        if not unsubscribe:
            print("ERROR: Subscription failed — device busy?")
            return
        print("Subscribed successfully.")

        while True:
            await asyncio.sleep(1)

    except (KeyboardInterrupt, asyncio.CancelledError):
        print("Stopping...")

    except Exception as e:
        print(f"ERROR during run: {e}")

    finally:
        if unsubscribe:
            print("Unsubscribing...")
            with contextlib.suppress(Exception):
                unsubscribe()
        print("Done.")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="ESPHome Bluetooth Proxy passive monitor (subscribe API)")
    parser.add_argument("hostname", help="ESPHome BTProxy hostname")
    parser.add_argument("--password", default="", help="API password")
    args = parser.parse_args()
    asyncio.run(main(args.hostname, args.password))
