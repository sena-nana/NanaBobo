use std::{
    io::Read,
    sync::{Arc, Mutex},
    time::Duration,
};

use brotli::Decompressor;
use flate2::read::ZlibDecoder;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tauri::{AppHandle, Emitter};
use tokio::sync::watch;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

use super::client::{BilibiliClient, DanmakuConnectionInfo};
use crate::models::{DanmakuConnection, DanmakuConnectionState, DanmakuMessage, DanmakuStatus};

pub const DANMAKU_MESSAGE_EVENT: &str = "nanabobo://danmaku/message";
pub const DANMAKU_STATUS_EVENT: &str = "nanabobo://danmaku/status";

const HEADER_SIZE: usize = 16;
const OP_HEARTBEAT: u32 = 2;
const OP_MESSAGE: u32 = 5;
const PROTO_RAW: u16 = 0;
const PROTO_HEARTBEAT: u16 = 1;
const PROTO_ZLIB: u16 = 2;
const PROTO_BROTLI: u16 = 3;

pub struct DanmakuManager {
    active: Mutex<Option<ActiveConnection>>,
    status: Mutex<DanmakuStatus>,
}

struct ActiveConnection {
    connection: DanmakuConnection,
    stop: watch::Sender<bool>,
    task: tauri::async_runtime::JoinHandle<()>,
}

impl Default for DanmakuManager {
    fn default() -> Self {
        Self {
            active: Mutex::new(None),
            status: Mutex::new(DanmakuStatus {
                connection_id: None,
                room_id: None,
                state: DanmakuConnectionState::Idle,
                message: None,
            }),
        }
    }
}

impl DanmakuManager {
    pub fn status(&self) -> DanmakuStatus {
        self.status
            .lock()
            .map(|status| status.clone())
            .unwrap_or(DanmakuStatus {
                connection_id: None,
                room_id: None,
                state: DanmakuConnectionState::Error,
                message: Some("弹幕连接状态暂时不可用。".to_owned()),
            })
    }

    pub fn start(
        self: &Arc<Self>,
        app: AppHandle,
        client: BilibiliClient,
        room_id: u64,
    ) -> Result<DanmakuConnection, String> {
        self.stop(&app, None);
        let connection = DanmakuConnection {
            connection_id: Uuid::new_v4().to_string(),
            room_id,
        };
        let (stop, stop_rx) = watch::channel(false);
        self.set_status(
            &app,
            DanmakuStatus {
                connection_id: Some(connection.connection_id.clone()),
                room_id: Some(room_id),
                state: DanmakuConnectionState::Connecting,
                message: None,
            },
        );
        let task_connection = connection.clone();
        let manager = Arc::clone(self);
        let task_app = app.clone();
        let task = tauri::async_runtime::spawn(async move {
            run_loop(manager, task_app, client, task_connection, stop_rx).await;
        });
        self.active
            .lock()
            .map_err(|_| "弹幕连接状态暂时不可用。".to_owned())?
            .replace(ActiveConnection {
                connection: connection.clone(),
                stop,
                task,
            });
        Ok(connection)
    }

    pub fn stop(&self, app: &AppHandle, connection_id: Option<&str>) -> bool {
        let active = self.active.lock().ok().and_then(|mut active| {
            let matches = active.as_ref().is_some_and(|current| {
                connection_id.is_none()
                    || connection_id == Some(current.connection.connection_id.as_str())
            });
            if matches {
                active.take()
            } else {
                None
            }
        });
        let Some(active) = active else { return false };
        let _ = active.stop.send(true);
        active.task.abort();
        self.set_status(
            app,
            DanmakuStatus {
                connection_id: None,
                room_id: None,
                state: DanmakuConnectionState::Stopped,
                message: None,
            },
        );
        true
    }

    fn set_status(&self, app: &AppHandle, status: DanmakuStatus) {
        if let Ok(mut current) = self.status.lock() {
            *current = status.clone();
        }
        let _ = app.emit(DANMAKU_STATUS_EVENT, status);
    }
}

async fn run_loop(
    manager: Arc<DanmakuManager>,
    app: AppHandle,
    client: BilibiliClient,
    connection: DanmakuConnection,
    mut stop: watch::Receiver<bool>,
) {
    let mut retry = 0usize;
    loop {
        if *stop.borrow() {
            return;
        }
        if retry > 0 {
            manager.set_status(
                &app,
                DanmakuStatus {
                    connection_id: Some(connection.connection_id.clone()),
                    room_id: Some(connection.room_id),
                    state: DanmakuConnectionState::Reconnecting,
                    message: Some("连接中断，正在重试。".to_owned()),
                },
            );
        }

        let info = match client.danmaku_info(connection.room_id).await {
            Ok(info) => info,
            Err(_) => {
                retry = retry.saturating_add(1).min(6);
                if !wait_before_retry(&mut stop, retry).await {
                    return;
                }
                continue;
            }
        };

        let mut connected = false;
        for host in &info.hosts {
            let endpoint = format!("wss://{}:{}/sub", host.host, host.wss_port);
            let Ok((socket, _)) = connect_async(endpoint).await else {
                continue;
            };
            if run_connection(&manager, &app, socket, &connection, &info, &mut stop)
                .await
                .is_ok()
            {
                if *stop.borrow() {
                    return;
                }
            }
            connected = true;
            break;
        }

        if *stop.borrow() {
            return;
        }
        retry = if connected {
            1
        } else {
            retry.saturating_add(1).min(6)
        };
        if !wait_before_retry(&mut stop, retry).await {
            return;
        }
    }
}

async fn wait_before_retry(stop: &mut watch::Receiver<bool>, retry: usize) -> bool {
    let seconds = [1, 2, 4, 8, 16, 30][retry.saturating_sub(1).min(5)];
    tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(seconds)) => true,
        changed = stop.changed() => changed.is_ok() && !*stop.borrow(),
    }
}

async fn run_connection(
    manager: &DanmakuManager,
    app: &AppHandle,
    socket: WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
    connection: &DanmakuConnection,
    info: &DanmakuConnectionInfo,
    stop: &mut watch::Receiver<bool>,
) -> Result<(), ()> {
    let (mut sink, mut stream) = socket.split();
    let auth_body = serde_json::json!({
        "uid": 0,
        "roomid": connection.room_id,
        "protover": 3,
        "platform": "web",
        "type": 2,
        "key": info.token,
    });
    let auth_payload = serde_json::to_vec(&auth_body).map_err(|_| ())?;
    sink.send(Message::Binary(
        build_packet(7, PROTO_RAW, &auth_payload).into(),
    ))
    .await
    .map_err(|_| ())?;
    manager.set_status(
        app,
        DanmakuStatus {
            connection_id: Some(connection.connection_id.clone()),
            room_id: Some(connection.room_id),
            state: DanmakuConnectionState::Connected,
            message: None,
        },
    );

    let mut heartbeat = tokio::time::interval(Duration::from_secs(30));
    loop {
        tokio::select! {
            changed = stop.changed() => {
                if changed.is_err() || *stop.borrow() { return Ok(()); }
            }
            _ = heartbeat.tick() => {
                sink.send(Message::Binary(build_packet(OP_HEARTBEAT, PROTO_HEARTBEAT, &[]).into())).await.map_err(|_| ())?;
            }
            incoming = stream.next() => match incoming {
                Some(Ok(Message::Binary(payload))) => {
                    for parsed in parse_frames(payload.as_ref()).map_err(|_| ())? {
                        let message = DanmakuMessage {
                            connection_id: connection.connection_id.clone(),
                            room_id: connection.room_id,
                            sender_name: parsed.sender_name,
                            text: parsed.text,
                            sent_at: now_seconds(),
                        };
                        let _ = app.emit(DANMAKU_MESSAGE_EVENT, message);
                    }
                }
                Some(Ok(Message::Text(payload))) => {
                    if let Some(parsed) = parse_chat_json(payload.as_bytes()) {
                        let _ = app.emit(DANMAKU_MESSAGE_EVENT, DanmakuMessage {
                            connection_id: connection.connection_id.clone(),
                            room_id: connection.room_id,
                            sender_name: parsed.sender_name,
                            text: parsed.text,
                            sent_at: now_seconds(),
                        });
                    }
                }
                Some(Ok(Message::Close(_))) | None => return Err(()),
                Some(Ok(_)) => {}
                Some(Err(_)) => return Err(()),
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ParsedMessage {
    sender_name: String,
    text: String,
}

fn parse_frames(mut bytes: &[u8]) -> Result<Vec<ParsedMessage>, ()> {
    let mut messages = Vec::new();
    while !bytes.is_empty() {
        if bytes.len() < HEADER_SIZE {
            return Err(());
        }
        let packet_size = u32::from_be_bytes(bytes[0..4].try_into().map_err(|_| ())?) as usize;
        let header_size = u16::from_be_bytes(bytes[4..6].try_into().map_err(|_| ())?) as usize;
        let protocol = u16::from_be_bytes(bytes[6..8].try_into().map_err(|_| ())?);
        let operation = u32::from_be_bytes(bytes[8..12].try_into().map_err(|_| ())?);
        if packet_size < header_size || header_size < HEADER_SIZE || packet_size > bytes.len() {
            return Err(());
        }
        let body = &bytes[header_size..packet_size];
        if operation == OP_MESSAGE {
            match protocol {
                PROTO_ZLIB => {
                    let mut decoder = ZlibDecoder::new(body);
                    let mut decoded = Vec::new();
                    decoder.read_to_end(&mut decoded).map_err(|_| ())?;
                    messages.extend(parse_frames(&decoded)?);
                }
                PROTO_BROTLI => {
                    let mut decoder = Decompressor::new(body, 4096);
                    let mut decoded = Vec::new();
                    decoder.read_to_end(&mut decoded).map_err(|_| ())?;
                    messages.extend(parse_frames(&decoded)?);
                }
                PROTO_RAW | PROTO_HEARTBEAT => {
                    if let Some(message) = parse_chat_json(body) {
                        messages.push(message);
                    }
                }
                _ => {}
            }
        }
        bytes = &bytes[packet_size..];
    }
    Ok(messages)
}

fn parse_chat_json(bytes: &[u8]) -> Option<ParsedMessage> {
    let value: Value = serde_json::from_slice(bytes).ok()?;
    if value.get("cmd")?.as_str()? != "DANMU_MSG" {
        return None;
    }
    let info = value.get("info")?.as_array()?;
    let text = info.get(1)?.as_str()?.trim();
    let sender_name = info.get(2)?.get(1)?.as_str()?.trim();
    if text.is_empty() || sender_name.is_empty() {
        return None;
    }
    Some(ParsedMessage {
        sender_name: sender_name.to_owned(),
        text: text.to_owned(),
    })
}

fn build_packet(operation: u32, protocol: u16, body: &[u8]) -> Vec<u8> {
    let packet_size = (HEADER_SIZE + body.len()) as u32;
    let mut packet = Vec::with_capacity(packet_size as usize);
    packet.extend_from_slice(&packet_size.to_be_bytes());
    packet.extend_from_slice(&(HEADER_SIZE as u16).to_be_bytes());
    packet.extend_from_slice(&protocol.to_be_bytes());
    packet.extend_from_slice(&operation.to_be_bytes());
    packet.extend_from_slice(&1u32.to_be_bytes());
    packet.extend_from_slice(body);
    packet
}

fn now_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use brotli::CompressorWriter;
    use flate2::{write::ZlibEncoder, Compression};

    use super::{build_packet, parse_frames, OP_MESSAGE, PROTO_RAW, PROTO_ZLIB};

    fn chat_packet() -> Vec<u8> {
        build_packet(
            OP_MESSAGE,
            PROTO_RAW,
            br#"{"cmd":"DANMU_MSG","info":[null,"\u4f60\u597d",[123,"\u89c2\u4f17"]]}"#,
        )
    }

    #[test]
    fn parses_public_chat_messages_without_exposing_raw_payload() {
        let messages = parse_frames(&chat_packet()).unwrap();
        assert_eq!(messages[0].sender_name, "观众");
        assert_eq!(messages[0].text, "你好");
    }

    #[test]
    fn parses_zlib_wrapped_frames() {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&chat_packet()).unwrap();
        let packet = build_packet(OP_MESSAGE, PROTO_ZLIB, &encoder.finish().unwrap());
        let messages = parse_frames(&packet).unwrap();
        assert_eq!(messages[0].text, "你好");
    }

    #[test]
    fn parses_brotli_wrapped_frames() {
        let mut encoded = Vec::new();
        {
            let mut writer = CompressorWriter::new(&mut encoded, 4096, 5, 22);
            writer.write_all(&chat_packet()).unwrap();
            writer.flush().unwrap();
        }
        let packet = super::build_packet(OP_MESSAGE, super::PROTO_BROTLI, &encoded);
        let messages = parse_frames(&packet).unwrap();
        assert_eq!(messages[0].sender_name, "观众");
    }

    #[test]
    fn rejects_truncated_frames() {
        assert!(parse_frames(&[0, 1, 2]).is_err());
    }
}
