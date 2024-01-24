# Experiment
Here follow the configurations and the steps needed to reproduce the main experiment conducted.

## Hardware
Four devices, one central (e.g. PC) and three edge (e.g. Raspberry), connected to the same LAN.

## Configuration
### Rust
- Each device must have the executable of the [`local-bridge`](/src/bin/local-bridge.rs) binary, compiled according to the device architecture.

### Python (3.11.2)
- Each device must have the [`bridge.py`](/py/bridge.py) python script.
- The central device must have the [`aggregator.py`](/py/aggregator.py) python script.
- Each edge device must have the [`trainer.py`](/py/trainer.py) python script.

### Dependencies
- Verify that every python dependency is satisfied on each device. It is suggested to create a virtual environment to better manage the necessary modules.

### TOML
- The central device must have the [`remote-bridge.toml`](/config/remote-bridge.toml) and [`remote-config.toml`](/config/remote-config.toml) configuration files.
- Each edge device must have one pair of `name-bridge.toml` and `name-config.toml` configuration files, where the name can be `alice`, `bob` or `charlie`.

### Adjustments
- Configure the `TOML` files of each device to match their name and IP address. Also verify that the chosen ports are available.

## Execution
- Execute the `local-bridge` program on each edge device, passing as the argument the path to the relative `name-bridge.toml`.
- Execute the `trainer.py` program on each device, passing or hardcoding the necessary parameters.
- Execute the `local-bridge` program on the central device, passing as the argument the path to `remote-bridge.toml`
- Execute the `aggregator.py` on the central device, passing or hardcoding the necessary parameters.
- Insert the training session option when prompted by `aggregator.py`
- wait and watch as the system does its work.

## Tips
- This version of the system is not ver lightweight, so it is recommended to use at least a Raspberry PI 4 or higher for acceptable performance. Raspberry PI 3 can be used, but it is necessary to increase its swap RAM.
- The parameters of `trainer.py` are: host_of_the_rust_bridge, port_of_the_rust_bridge, number_of_edge_device (1, 2, 3), total_of_edge_devices (3), name (alice, bob, charlie)
- The parameters of `aggregator.py` are: host_of_the_rust_bridge, port_of_the_rust_bridge