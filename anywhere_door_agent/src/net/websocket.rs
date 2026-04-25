use std::path::PathBuf;
use std::time::Duration;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::watch;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use crate::net::net::NetworkService;
use crate::auth::AuthService;

#[derive(Debug, Deserialize)]
struct SyncEvent {
    #[serde(rename = "type")]
    event_type_str: String,
    event_type: Option<String>,
    file_id: Option<String>,
    file_name: Option<String>,
    file_path: Option<String>,
    file_size: Option<u64>,
    source: Option<String>,
}

fn get_primary_sync_root() -> PathBuf {
    let roots_str = std::env::var("ANYWHERE_DOOR_WATCH_ROOTS").unwrap_or_else(|_| ".".to_string());
    let first_root = roots_str.split(',').next().unwrap_or(".");
    PathBuf::from(first_root)
}

fn server_url() -> String {
    std::env::var("ANYWHERE_DOOR_SERVER_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8000".to_string())
}

pub fn spawn_websocket_listener(
    mut stop: watch::Receiver<bool>,
    credentials_path: PathBuf,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let base_url = server_url();
        let sync_root = get_primary_sync_root();

        loop {
            if *stop.borrow() {
                break;
            }

            // Load credentials and JWT
            let (device_id, device_secret, jwt) = match NetworkService::load_device_credentials(&credentials_path) {
                Ok(creds) => creds,
                Err(e) => {
                    eprintln!("WebSocket: failed to load credentials: {}", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            let ws_url = base_url.replace("http://", "ws://").replace("https://", "wss://");
            let encoded_jwt: String = url::form_urlencoded::byte_serialize(jwt.as_bytes()).collect();
            let ws_url = format!("{}/ws/sync/{}", ws_url, encoded_jwt);

            eprintln!("WebSocket: connecting to server...");
            
            match connect_async(&ws_url).await {
                Ok((mut ws_stream, _)) => {
                    eprintln!("WebSocket: connected successfully");
                    
                    let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(30));
                    
                    loop {
                        tokio::select! {
                            _ = heartbeat_interval.tick() => {
                                let _ = ws_stream.send(Message::Text("ping".into())).await;
                            }
                            msg = ws_stream.next() => {
                                match msg {
                                    Some(Ok(Message::Text(text))) => {
                                        if text == "pong" {
                                            continue;
                                        }
                                        if let Ok(event) = serde_json::from_str::<SyncEvent>(&text) {
                                            if event.event_type_str == "file_event" {
                                                if let (Some(ev_type), Some(source), Some(file_id), Some(file_path)) = 
                                                    (&event.event_type, &event.source, &event.file_id, &event.file_path) {
                                                    
                                                    if source == "frontend" {
                                                        // Construct NetworkService just to download
                                                        let auth_service = AuthService::new("jwt-secret".to_string());
                                                        let net_service = NetworkService::new(
                                                            base_url.clone(),
                                                            auth_service,
                                                            jwt.clone(),
                                                            device_id.clone(),
                                                            device_secret.clone(),
                                                        );

                                                        // Handle paths correctly
                                                        // If file_path is absolute, join() will use the absolute path
                                                        // If relative, it will be placed inside sync_root
                                                        let mut target_path = sync_root.join(file_path);
                                                        
                                                        // Check if file_path actually contains a path with '/' or '\'
                                                        // For frontend uploads, it might just be the filename.
                                                        if let Some(name) = &event.file_name {
                                                            if file_path == name {
                                                                // It's just a filename, put it in sync root
                                                                target_path = sync_root.join(name);
                                                            }
                                                        }

                                                        if ev_type == "upload" {
                                                            eprintln!("WebSocket: downloading {} from frontend", target_path.display());
                                                            if let Err(e) = net_service.download_file(file_id, &target_path).await {
                                                                eprintln!("WebSocket: failed to download {}: {}", target_path.display(), e);
                                                            } else {
                                                                eprintln!("WebSocket: successfully downloaded {}", target_path.display());
                                                            }
                                                        } else if ev_type == "delete" {
                                                            eprintln!("WebSocket: deleting {}", target_path.display());
                                                            if let Err(e) = NetworkService::delete_local_file(&target_path) {
                                                                eprintln!("WebSocket: failed to delete {}: {}", target_path.display(), e);
                                                            } else {
                                                                eprintln!("WebSocket: successfully deleted {}", target_path.display());
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Some(Ok(Message::Close(_))) | None => {
                                        eprintln!("WebSocket: connection closed");
                                        break;
                                    }
                                    Some(Err(e)) => {
                                        eprintln!("WebSocket: error: {}", e);
                                        break;
                                    }
                                    _ => {}
                                }
                            }
                            _ = stop.changed() => {
                                let _ = ws_stream.close(None).await;
                                return;
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("WebSocket: connection failed: {}", e);
                }
            }
            
            // Retry delay
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(5)) => {}
                _ = stop.changed() => { break; }
            }
        }
    })
}
