Linux Bluetooth Proxy for ESPHome
=================================

This project provides a Bluetooth proxy daemon for ESPHome, designed to run on Linux systems. It listens for Bluetooth Low Energy (BLE) advertisements using the BlueZ stack and forwards them over TCP to ESPHome or other compatible clients. The proxy also advertises itself via mDNS as esphomelib for easy network discovery.

Current version cooperates with desktop and other system usage of the bluetooth hardware by using the bluez stack via dbus. Future work to access raw advertisements via
HCI, bypassing any filtering or delay that bluez may be doing is being considered.

Usage
-----
System packages for debian based systems (Debian. Ubuntu, Pop-OS) are provided as part of the release package. This daemon runs as a systemd unit.


For testing and development, you many run the proxy daemon with:

.. code-block:: bash

   cargo run --release -- [OPTIONS]

Options:

- ``-a, --hci <INDEX>``: Bluetooth adapter index (default: 0 for hci0)
- ``-l, --listen <ADDR>``: TCP listen address (default: 0.0.0.0:6053)
- ``--hostname <NAME>``: Hostname to advertise (default: system hostname)
- ``-m, --mac <MAC>``: MAC address for mDNS (optional)

Example:

.. code-block:: bash

   cargo run --release -- --hci 1 --listen 192.168.1.10:6053 --hostname my-bt-proxy

Building
--------

Requires Rust (edition 2021 or newer) and a Linux system with BlueZ.

.. code-block:: bash

   cargo build --release

Project Structure
-----------------

- ``src/main.rs``: Entry point and CLI handling
- ``src/ble.rs``: BLE advertisement listener logic
- ``src/mdns.rs``: mDNS service registration
- ``src/server.rs``: TCP server implementation
- ``src/context.rs``: Shared proxy context
- ``src/utils.rs``: Utility functions

License
-------

This project is licensed under the GPL 3.0 or later.

Contributing
------------

Pull requests and issues are welcome! Please open an issue for bug reports or feature requests.
