use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::Manager;
use futures::executor::block_on;
use futures::StreamExt;
use uuid::Uuid;
use std::time::Duration;
use std::thread::sleep;
use tokio::runtime::Runtime;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::accept_async;
use tokio::net::TcpListener;
use std::sync::{Arc, Mutex};
use futures::{SinkExt};
use serde::Serialize;

#[derive(Serialize, Clone)]
struct HrPacket {
    hr: u16,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mac_arg = args.iter().position(|x| x == "--mac").and_then(|i| args.get(i + 1)).cloned();
    let rt = Runtime::new().unwrap();
    rt.block_on(async_main(mac_arg));
}

async fn async_main(mac_arg: Option<String>) {
    let hr_value = Arc::new(Mutex::new(0u16));
    let hr_value_ws = hr_value.clone();
    // Spawn WebSocket server
    tokio::spawn(async move {
        let listener = TcpListener::bind("127.0.0.1:9001").await.expect("Failed to bind WebSocket port");
        // println!("WebSocket server running on ws://127.0.0.1:9001");
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let hr_value = hr_value_ws.clone();
            tokio::spawn(async move {
                let mut ws_stream = accept_async(stream).await.unwrap();
                loop {
                    let hr = *hr_value.lock().unwrap();
                    let packet = HrPacket { hr };
                    let msg = serde_json::to_string(&packet).unwrap();
                    if ws_stream.send(Message::Text(msg)).await.is_err() {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(1000)).await;
                }
            });
        }
    });
    // BLE logic
    run_ble(hr_value, mac_arg).await;
}

async fn run_ble(hr_value: Arc<Mutex<u16>>, mac_arg: Option<String>) {
    // println!("Scanning for Polar H10 (Heart Rate Monitor)...");
    let manager = Manager::new().await.unwrap();
    let adapters = manager.adapters().await.unwrap();
    if adapters.is_empty() {
        // eprintln!("No Bluetooth adapters found");
        return;
    }
    let central = adapters.into_iter().nth(0).unwrap();
    central.start_scan(ScanFilter::default()).await.unwrap();
    sleep(Duration::from_secs(2));
    let peripherals = central.peripherals().await.unwrap();
    let hr_char = Uuid::parse_str("00002a37-0000-1000-8000-00805f9b34fb").unwrap();
    let mut found = false;
    for p in peripherals.iter() {
        let props = p.properties().await.unwrap();
        let local_name = props.as_ref().and_then(|x| x.local_name.clone());
        let address = props.as_ref().map(|x| x.address.clone());
        if let Some(mac) = &mac_arg {
            // Try to match by MAC address (case-insensitive, ignore colons)
            if let Some(addr) = &address {
                let mac_clean = mac.replace(":", "").to_lowercase();
                let addr_str = addr.to_string();
                let addr_clean = addr_str.replace(":", "").to_lowercase();
                if mac_clean == addr_clean {
                    // println!("Found device by MAC: {}", addr);
                    found = true;
                    p.connect().await.unwrap();
                    p.discover_services().await.unwrap();
                    let chars = p.characteristics();
                    let hr_measurement = chars.iter().find(|c| c.uuid == hr_char && c.properties.contains(btleplug::api::CharPropFlags::NOTIFY));
                    if let Some(char) = hr_measurement {
                        let mut notification_stream = p.notifications().await.unwrap();
                        p.subscribe(char).await.unwrap();
                        // println!("Subscribed to heart rate notifications.");
                        while let Some(data) = notification_stream.next().await {
                            if data.uuid == hr_char {
                                if let Some(hr) = parse_heart_rate(&data.value) {
                                    // println!("Heart Rate: {} bpm", hr);
                                    *hr_value.lock().unwrap() = hr;
                                }
                            }
                        }
                        // println!("Device disconnected or notification stream ended.");
                        *hr_value.lock().unwrap() = 0;
                    } else {
                        // eprintln!("Heart Rate Measurement characteristic not found!");
                    }
                    break;
                }
            }
        } else if let Some(name) = local_name {
            if name.to_lowercase().contains("polar") {
                // println!("Found device: {}", name);
                found = true;
                p.connect().await.unwrap();
                p.discover_services().await.unwrap();
                let chars = p.characteristics();
                let hr_measurement = chars.iter().find(|c| c.uuid == hr_char && c.properties.contains(btleplug::api::CharPropFlags::NOTIFY));
                if let Some(char) = hr_measurement {
                    let mut notification_stream = p.notifications().await.unwrap();
                    p.subscribe(char).await.unwrap();
                    // println!("Subscribed to heart rate notifications.");
                    while let Some(data) = notification_stream.next().await {
                        if data.uuid == hr_char {
                            if let Some(hr) = parse_heart_rate(&data.value) {
                                // println!("Heart Rate: {} bpm", hr);
                                *hr_value.lock().unwrap() = hr;
                            }
                        }
                    }
                    // println!("Device disconnected or notification stream ended.");
                    *hr_value.lock().unwrap() = 0;
                } else {
                    // eprintln!("Heart Rate Measurement characteristic not found!");
                }
                break;
            }
        }
    }
    if !found {
        // if mac_arg.is_some() {
        //     eprintln!("Device with specified MAC not found. Make sure it is active and in range.");
        // } else {
        //     eprintln!("Polar H10 not found. Make sure it is active and in range.");
        // }
    }
}

fn parse_heart_rate(data: &[u8]) -> Option<u16> {
    if data.len() < 2 { return None; }
    let flags = data[0];
    if flags & 0x01 == 0 {
        Some(data[1] as u16)
    } else if data.len() >= 3 {
        Some(u16::from_le_bytes([data[1], data[2]]))
    } else {
        None
    }
}
