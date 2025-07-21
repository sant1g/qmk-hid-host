use serde::Deserialize;
use std::io::Error;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::data_type::DataType;

use super::super::_base::Provider;

#[derive(Deserialize)]
struct MediaData {
    artist: String,
    title: String,
    bundleIdentifier: String,
    playing: bool,
}

fn send_media_data(metadata: Vec<String>, data_sender: &broadcast::Sender<Vec<u8>>, current: &(String, String)) -> (String, String) {
    let (mut artist, mut title) = current.clone();

    artist = metadata.get(0).unwrap().clone();
    if current.0 != artist {
        send_data(DataType::MediaArtist, &artist, &data_sender);
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    title = metadata.get(1).unwrap().clone();
    if current.1 != title {
        send_data(DataType::MediaTitle, &title, &data_sender);
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    (artist, title)
}

fn parse_media_data(raw: &str) -> Result<MediaData, Error> {
    let data = serde_json::from_str(raw)?;

    Ok(data)
}

fn send_data(data_type: DataType, value: &String, data_sender: &broadcast::Sender<Vec<u8>>) {
    let mut data = value.to_string().into_bytes();
    data.truncate(30);
    data.insert(0, data.len() as u8);
    data.insert(0, data_type as u8);
    data_sender.send(data).unwrap();
}

pub struct MediaProvider {
    data_sender: broadcast::Sender<Vec<u8>>,
    is_started: Arc<AtomicBool>,
}

impl MediaProvider {
    pub fn new(data_sender: broadcast::Sender<Vec<u8>>) -> Box<dyn Provider> {
        let provider = MediaProvider {
            data_sender,
            is_started: Arc::new(AtomicBool::new(false)),
        };
        return Box::new(provider);
    }
}

impl Provider for MediaProvider {
    fn start(&self) {
        tracing::info!("Media Provider started");
        self.is_started.store(true, Relaxed);
        let data_sender = self.data_sender.clone();
        let is_started = self.is_started.clone();
        std::thread::spawn(move || {
            let mut media_data = (String::default(), String::default());

            loop {
                if !is_started.load(Relaxed) {
                    break;
                }

                let mut vec: Vec<String> = Vec::new();

                let output_str = Command::new("media-control").arg("get").output().unwrap().stdout;

                let json_string = String::from_utf8_lossy(&output_str).to_string();

                if json_string.is_empty() || json_string == "null\n" {
                    vec.push("No Artist".to_string());
                    vec.push("No Title".to_string());
                } else {
                    let md = parse_media_data(&json_string).unwrap();
                    vec.push(md.artist);
                    vec.push(md.title);
                }

                media_data = send_media_data(vec, &data_sender, &media_data);

                std::thread::sleep(std::time::Duration::from_millis(1_000));
            }

            tracing::info!("Media Provider stopped");
        });
    }

    fn stop(&self) {
        self.is_started.store(false, Relaxed);
    }
}
