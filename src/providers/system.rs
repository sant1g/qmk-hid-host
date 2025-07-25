use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::Arc;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, Networks, RefreshKind, System};
use tokio::sync::broadcast;

use crate::data_type::DataType;

use super::_base::Provider;

fn build_refresh_kind() -> RefreshKind {
    RefreshKind::nothing()
        .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
        .with_memory(MemoryRefreshKind::nothing().with_ram().with_swap())
}

fn get_cpu_usage(s: &System) -> u32 {
    let cpu_count = s.cpus().len() as f32;

    (s.global_cpu_usage() / cpu_count).round() as u32
}

fn get_memory_usage(s: &System) -> u32 {
    let ram_total = s.total_memory();
    let ram_used = s.used_memory();

    if ram_total > 0 {
        ((ram_used as f32 / ram_total as f32) * 100.0).round() as u32
    } else {
        0
    }
}

fn get_network_usage(n: &Networks) -> (u32, u32) {
    let tx: u64 = n.iter().map(|n| n.1.transmitted()).sum();
    let rx: u64 = n.iter().map(|n| n.1.received()).sum();

    ((rx / 1024 / 1024) as u32, (tx / 1024 / 1024) as u32)
}

fn send_system_data(data: Vec<u32>, data_sender: &broadcast::Sender<Vec<u8>>) {
    let cpu_data = data.get(0).unwrap().clone();
    send_data(DataType::CPUUsage, cpu_data, data_sender);
    std::thread::sleep(std::time::Duration::from_millis(50));

    let memory_data = data.get(1).unwrap().clone();
    send_data(DataType::RAMUsage, memory_data, data_sender);
    std::thread::sleep(std::time::Duration::from_millis(50));

    let network_rx = data.get(2).unwrap().clone();
    send_data(DataType::NetworkRX, network_rx, data_sender);
    std::thread::sleep(std::time::Duration::from_millis(50));

    let network_tx = data.get(3).unwrap().clone();
    send_data(DataType::NetworkTX, network_tx, data_sender);
    std::thread::sleep(std::time::Duration::from_millis(50));
}

fn send_data(data_type: DataType, value: u32, data_sender: &broadcast::Sender<Vec<u8>>) {
    data_sender.send(vec![data_type as u8, value as u8]).unwrap();
}

pub struct SystemProvider {
    host_to_device_sender: broadcast::Sender<Vec<u8>>,
    is_started: Arc<AtomicBool>,
}

impl SystemProvider {
    pub fn new(host_to_device_sender: broadcast::Sender<Vec<u8>>) -> Box<dyn Provider> {
        let provider = SystemProvider {
            host_to_device_sender,
            is_started: Arc::new(AtomicBool::new(false)),
        };
        return Box::new(provider);
    }
}

impl Provider for SystemProvider {
    fn start(&self) {
        tracing::info!("System Provider started");
        self.is_started.store(true, Relaxed);
        let data_sender = self.host_to_device_sender.clone();
        let is_started = self.is_started.clone();
        std::thread::spawn(move || {
            let refresh_kind = build_refresh_kind();
            let mut system = System::new_with_specifics(refresh_kind);
            let mut networks = Networks::new_with_refreshed_list();

            loop {
                if !is_started.load(Relaxed) {
                    break;
                }

                system.refresh_specifics(refresh_kind);
                networks.refresh(true);

                let cpu_usage = get_cpu_usage(&system);
                let memory_usage = get_memory_usage(&system);
                let network_usage = get_network_usage(&networks);

                let data = vec![cpu_usage, memory_usage, network_usage.0, network_usage.1];
                send_system_data(data, &data_sender);

                std::thread::sleep(std::time::Duration::from_millis(1_000));
            }

            tracing::info!("System Provider stopped");
        });
    }

    fn stop(&self) {
        self.is_started.store(false, Relaxed);
    }
}
