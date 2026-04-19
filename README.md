# Polar H10 Live Heart Rate (Rust + Web UI)

Real-time heart rate streaming from a **Polar H10** chest strap using **Bluetooth Low Energy (BLE)**, written in **Rust**, with a lightweight **WebSocket server** and a simple **HTML dashboard** to visualize your live BPM.

## What it does

- Scans for a Polar H10 (or connects to a specific device by MAC)
- Subscribes to the standard BLE Heart Rate Measurement characteristic (`0x2A37`)
- Keeps the latest heart rate in memory
- Broadcasts the latest value once per second over WebSocket:
  - `ws://127.0.0.1:9001`
  - payload format: `{"hr": <number>}`
- Provides a dashboard (`hr_interface.html`) that connects to the WebSocket and displays:
  - current heart rate (BPM)
  - average + max (session)
  - a gauge with configurable “zones”

## Project layout

- `main.rs` — BLE scanner/connector + WebSocket server
- `hr_interface.html` — standalone UI (open it in a browser)
- `README` (current) — original quick notes

## Requirements

- Rust toolchain (stable is fine)
- A working Bluetooth adapter
- Permissions to use BLE on your OS
- A Polar H10 (active and worn)

## Run

### 1) Build the Rust app
```bash
cargo build
```

### 2) Start the Rust app

```bash
cargo run
```

By default it scans for a device with “polar” in its advertised name, connects, and begins streaming.

### 3) Open the dashboard

Open `hr_interface.html` in your browser (double-click it, or “Open File…”).

It will attempt to connect to:

- `ws://localhost:9001`

If the app is running, the UI should switch to **Connected & Syncing** and show your BPM.

## Connecting to a specific Polar H10 (by MAC)

If scanning is unreliable (multiple devices, hidden names, etc.), you can target a device address:

```bash
cargo run -- --mac AA:BB:CC:DD:EE:FF (your MAC address here)
```

## Troubleshooting

### “No Bluetooth adapters found”
- Your system Bluetooth may be disabled, missing drivers, or blocked by permissions.

### “Polar not found”
- Make sure the strap is worn (the H10 often won’t advertise/stream reliably if it isn’t detecting contact).
- Bring it close to the computer.
- Try specifying `--mac ...` if scanning doesn’t pick it up.

### Linux permissions
BLE access on Linux often requires extra setup depending on distro / BlueZ configuration.

If you run into permission issues, you may need elevated privileges or capabilities. One example you may see online is:

```bash
sudo setcap 'cap_net_raw,cap_net_admin+eip' $(which bluetoothd)
```

(Exact steps vary by distro—if you tell me your OS, I can tailor this section.)

### UI says “Disconnected – Waiting…”
- Confirm the Rust app is running.
- Confirm nothing else is already using port `9001`.
- The UI expects JSON like `{"hr":123}` once per second; it treats `0` as disconnected.

## How it works (quick overview)

- **BLE:** subscribes to Heart Rate Measurement notifications (UUID `00002a37-0000-1000-8000-00805f9b34fb`)
- **Parsing:** interprets the flag byte to determine whether the HR is 8-bit or 16-bit
- **WebSocket:** sends the latest HR value once per second to each connected client

## Safety / disclaimer

This project is for hobby/fitness visualization and experimentation. It is **not** medical software.
