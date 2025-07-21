use chrono::{DateTime, Datelike, Local};
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::data_type::DataType;

use super::_base::Provider;

fn get_date() -> (u8, u8) {
    let now: DateTime<Local> = Local::now();
    let day = now.day() as u8;
    let month = now.month() as u8;

    return (day, month);
}

fn send_data(value: &(u8, u8), host_to_device_sender: &broadcast::Sender<Vec<u8>>) {
    let data = vec![DataType::Date as u8, value.0, value.1];
    if let Err(e) = host_to_device_sender.send(data) {
        tracing::error!("Date Provider failed to send data: {:}", e);
    }
}

pub struct DateProvider {
    host_to_device_sender: broadcast::Sender<Vec<u8>>,
    is_started: Arc<AtomicBool>,
}

impl DateProvider {
    pub fn new(host_to_device_sender: broadcast::Sender<Vec<u8>>) -> Box<dyn Provider> {
        let provider = DateProvider {
            host_to_device_sender,
            is_started: Arc::new(AtomicBool::new(false)),
        };
        return Box::new(provider);
    }
}

impl Provider for DateProvider {
    fn start(&self) {
        tracing::info!("Date Provider started");
        self.is_started.store(true, Relaxed);
        let host_to_device_sender = self.host_to_device_sender.clone();
        let is_started = self.is_started.clone();
        std::thread::spawn(move || {
            let mut synced_date = (0u8, 0u8);
            loop {
                if !is_started.load(Relaxed) {
                    break;
                }

                let date = get_date();
                if synced_date != date {
                    synced_date = date;
                    send_data(&date, &host_to_device_sender);
                }

                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            tracing::info!("Date Provider stopped");
        });
    }

    fn stop(&self) {
        self.is_started.store(false, Relaxed);
    }
}
