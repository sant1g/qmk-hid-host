use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use tokio::sync::broadcast;
use crate::consumers::_base::Consumer;
use crate::data_type::DataType;
use crate::providers::relay::RelayProvider;

pub struct EncoderConsumer {
    device_to_host_sender: broadcast::Sender<Vec<u8>>,
    is_started: Arc<AtomicBool>,
}

impl EncoderConsumer {
    pub fn new(device_to_host_sender: broadcast::Sender<Vec<u8>>) -> Box<dyn Consumer> {
        let consumer = EncoderConsumer {
            device_to_host_sender,
            is_started: Arc::new(AtomicBool::new(false)),
        };
        return Box::new(consumer);
    }
}

impl Consumer for EncoderConsumer {
    fn start(&self) {
        tracing::info!("Encoder Consumer started");
        self.is_started.store(true, Relaxed);
        let is_started = self.is_started.clone();
        let mut subscriber = self.device_to_host_sender.subscribe();
        std::thread::spawn(move || {
            loop {
                if !is_started.load(Relaxed) {
                    break;
                }

                tracing::debug!("Encoder Consumer: waiting for data...");
                if let Ok(mut data) = subscriber.blocking_recv() {
                    if data[0] == DataType::EncoderMode as u8 {
                        tracing::debug!("Encoder Consumer: got data: {:?}", data);
                        let out = Command::new("sketchybar").arg("--trigger").arg("encoder_mode").arg(format!("INDEX={}", data[1])).arg(format!("MODE={}", data[2])).output().unwrap();
                        tracing::debug!("Encoder Consumer: wrote data: {:?}", out);
                    }
                }

                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            tracing::info!("Encoder Consumer stopped");
        });
    }

    fn stop(&self) {
        self.is_started.store(false, Relaxed);
    }
}
