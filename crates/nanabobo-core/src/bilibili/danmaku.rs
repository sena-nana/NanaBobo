use std::{
    io::Read,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use brotli::Decompressor;
use flate2::read::ZlibDecoder;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::watch;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

use super::client::{BilibiliClient, DanmakuConnectionInfo};
use crate::events::EventSink;
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

const DANMAKU_PARSE_LIMITS: ParseLimits = ParseLimits {
    max_input_bytes: 1024 * 1024,
    max_expanded_bytes_per_frame: 2 * 1024 * 1024,
    max_total_expanded_bytes: 4 * 1024 * 1024,
    max_nested_depth: 4,
    max_frames: 1024,
    max_messages: 256,
    max_sender_name_bytes: 256,
    max_message_text_bytes: 4 * 1024,
};

#[derive(Clone, Copy)]
struct ParseLimits {
    max_input_bytes: usize,
    max_expanded_bytes_per_frame: usize,
    max_total_expanded_bytes: usize,
    max_nested_depth: usize,
    max_frames: usize,
    max_messages: usize,
    max_sender_name_bytes: usize,
    max_message_text_bytes: usize,
}

pub struct DanmakuManager {
    runtime: tokio::runtime::Handle,
    sink: Arc<dyn EventSink>,
    active: Mutex<Option<ActiveConnection>>,
    status: Mutex<DanmakuStatus>,
}

struct ActiveConnection {
    connection: DanmakuConnection,
    stop: watch::Sender<bool>,
    task: tokio::task::JoinHandle<()>,
}

impl DanmakuManager {
    pub fn new(runtime: tokio::runtime::Handle, sink: Arc<dyn EventSink>) -> Self {
        Self {
            runtime,
            sink,
            active: Mutex::new(None),
            status: Mutex::new(DanmakuStatus {
                connection_id: None,
                room_id: None,
                state: DanmakuConnectionState::Idle,
                message: None,
            }),
        }
    }

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
        client: BilibiliClient,
        room_id: u64,
    ) -> Result<DanmakuConnection, String> {
        self.stop(None);
        let connection = DanmakuConnection {
            connection_id: Uuid::new_v4().to_string(),
            room_id,
        };
        let (stop, stop_rx) = watch::channel(false);
        self.set_status(DanmakuStatus {
            connection_id: Some(connection.connection_id.clone()),
            room_id: Some(room_id),
            state: DanmakuConnectionState::Connecting,
            message: None,
        });
        let task_connection = connection.clone();
        let sink = Arc::clone(&self.sink);
        let task = self.runtime.spawn(async move {
            run_loop(sink, client, task_connection, stop_rx).await;
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

    pub fn stop(&self, connection_id: Option<&str>) -> bool {
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
        self.set_status(DanmakuStatus {
            connection_id: None,
            room_id: None,
            state: DanmakuConnectionState::Stopped,
            message: None,
        });
        true
    }

    fn set_status(&self, status: DanmakuStatus) {
        if let Ok(mut current) = self.status.lock() {
            *current = status.clone();
        }
        self.emit(DANMAKU_STATUS_EVENT, &status);
    }

    fn emit<E: serde::Serialize>(&self, event: &str, payload: &E) {
        let Ok(value) = serde_json::to_value(payload) else {
            return;
        };
        self.sink.emit(event, value);
    }
}

async fn run_loop(
    sink: Arc<dyn EventSink>,
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
            emit_status(
                &sink,
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
            if run_connection(&sink, socket, &connection, &info, &mut stop)
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

fn emit_status(sink: &Arc<dyn EventSink>, status: DanmakuStatus) {
    let Ok(value) = serde_json::to_value(&status) else {
        return;
    };
    sink.emit(DANMAKU_STATUS_EVENT, value);
}

fn emit_message(sink: &Arc<dyn EventSink>, connection: &DanmakuConnection, parsed: ParsedMessage) {
    let message = DanmakuMessage {
        connection_id: connection.connection_id.clone(),
        room_id: connection.room_id,
        sender_name: parsed.sender_name,
        text: parsed.text,
        sent_at: now_seconds(),
    };
    let Ok(value) = serde_json::to_value(&message) else {
        return;
    };
    sink.emit(DANMAKU_MESSAGE_EVENT, value);
}

async fn wait_before_retry(stop: &mut watch::Receiver<bool>, retry: usize) -> bool {
    let seconds = [1, 2, 4, 8, 16, 30][retry.saturating_sub(1).min(5)];
    tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(seconds)) => true,
        changed = stop.changed() => changed.is_ok() && !*stop.borrow(),
    }
}

async fn run_connection(
    sink: &Arc<dyn EventSink>,
    socket: WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
    connection: &DanmakuConnection,
    info: &DanmakuConnectionInfo,
    stop: &mut watch::Receiver<bool>,
) -> Result<(), ()> {
    let (mut sink_io, mut stream) = socket.split();
    let auth_body = serde_json::json!({
        "uid": 0,
        "roomid": connection.room_id,
        "protover": 3,
        "platform": "web",
        "type": 2,
        "key": info.token,
    });
    let auth_payload = serde_json::to_vec(&auth_body).map_err(|_| ())?;
    sink_io
        .send(Message::Binary(
            build_packet(7, PROTO_RAW, &auth_payload).into(),
        ))
        .await
        .map_err(|_| ())?;
    emit_status(
        sink,
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
                sink_io.send(Message::Binary(build_packet(OP_HEARTBEAT, PROTO_HEARTBEAT, &[]).into())).await.map_err(|_| ())?;
            }
            incoming = stream.next() => match incoming {
                Some(Ok(Message::Binary(payload))) => {
                    match parse_frames(payload.as_ref()) {
                        Ok(messages) => {
                            for parsed in messages {
                                emit_message(sink, &connection, parsed);
                            }
                        }
                        Err(error) => {
                            report_parse_rejection(&error);
                            if !error.is_budget_exceeded() {
                                return Err(());
                            }
                        }
                    }
                }
                Some(Ok(Message::Text(payload))) => {
                    match parse_text_message(payload.as_bytes()) {
                        Ok(Some(parsed)) => emit_message(sink, &connection, parsed),
                        Ok(None) => {}
                        Err(error) => report_parse_rejection(&error),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BudgetLimit {
    InputBytes,
    ExpandedFrameBytes,
    ExpandedTotalBytes,
    NestedDepth,
    Frames,
    Messages,
    SenderNameBytes,
    MessageTextBytes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParseFailure {
    Budget(BudgetLimit),
    InvalidFrame,
    Decompression,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ParseStats {
    input_bytes: usize,
    expanded_bytes: usize,
    max_depth: usize,
    frames: usize,
    messages: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ParseError {
    failure: ParseFailure,
    stats: ParseStats,
    elapsed: Duration,
}

impl ParseError {
    fn is_budget_exceeded(&self) -> bool {
        matches!(self.failure, ParseFailure::Budget(_))
    }
}

struct ParseBudget {
    limits: ParseLimits,
    stats: ParseStats,
}

impl ParseBudget {
    fn record_depth(&mut self, depth: usize) -> Result<(), ParseFailure> {
        self.stats.max_depth = self.stats.max_depth.max(depth);
        if depth > self.limits.max_nested_depth {
            return Err(ParseFailure::Budget(BudgetLimit::NestedDepth));
        }
        Ok(())
    }

    fn record_frame(&mut self) -> Result<(), ParseFailure> {
        self.stats.frames = self.stats.frames.saturating_add(1);
        if self.stats.frames > self.limits.max_frames {
            return Err(ParseFailure::Budget(BudgetLimit::Frames));
        }
        Ok(())
    }

    fn record_message(&mut self) -> Result<(), ParseFailure> {
        self.stats.messages = self.stats.messages.saturating_add(1);
        if self.stats.messages > self.limits.max_messages {
            return Err(ParseFailure::Budget(BudgetLimit::Messages));
        }
        Ok(())
    }
}

fn parse_frames(bytes: &[u8]) -> Result<Vec<ParsedMessage>, ParseError> {
    parse_frames_with_limits(bytes, DANMAKU_PARSE_LIMITS)
}

fn parse_frames_with_limits(
    bytes: &[u8],
    limits: ParseLimits,
) -> Result<Vec<ParsedMessage>, ParseError> {
    parse_with_budget(bytes, limits, |budget| {
        let mut messages = Vec::new();
        parse_frames_inner(bytes, 0, budget, &mut messages)?;
        Ok(messages)
    })
}

fn parse_text_message(bytes: &[u8]) -> Result<Option<ParsedMessage>, ParseError> {
    parse_with_budget(bytes, DANMAKU_PARSE_LIMITS, |budget| {
        parse_chat_json(bytes, budget)
    })
}

fn parse_with_budget<T>(
    bytes: &[u8],
    limits: ParseLimits,
    parse: impl FnOnce(&mut ParseBudget) -> Result<T, ParseFailure>,
) -> Result<T, ParseError> {
    let started = Instant::now();
    let mut budget = ParseBudget {
        limits,
        stats: ParseStats {
            input_bytes: bytes.len(),
            expanded_bytes: 0,
            max_depth: 0,
            frames: 0,
            messages: 0,
        },
    };
    if bytes.len() > limits.max_input_bytes {
        return Err(ParseError {
            failure: ParseFailure::Budget(BudgetLimit::InputBytes),
            stats: budget.stats,
            elapsed: started.elapsed(),
        });
    }
    parse(&mut budget).map_err(|failure| ParseError {
        failure,
        stats: budget.stats,
        elapsed: started.elapsed(),
    })
}

fn parse_frames_inner(
    mut bytes: &[u8],
    depth: usize,
    budget: &mut ParseBudget,
    messages: &mut Vec<ParsedMessage>,
) -> Result<(), ParseFailure> {
    while !bytes.is_empty() {
        if bytes.len() < HEADER_SIZE {
            return Err(ParseFailure::InvalidFrame);
        }
        let packet_size = u32::from_be_bytes(
            bytes[0..4]
                .try_into()
                .map_err(|_| ParseFailure::InvalidFrame)?,
        ) as usize;
        let header_size = u16::from_be_bytes(
            bytes[4..6]
                .try_into()
                .map_err(|_| ParseFailure::InvalidFrame)?,
        ) as usize;
        let protocol = u16::from_be_bytes(
            bytes[6..8]
                .try_into()
                .map_err(|_| ParseFailure::InvalidFrame)?,
        );
        let operation = u32::from_be_bytes(
            bytes[8..12]
                .try_into()
                .map_err(|_| ParseFailure::InvalidFrame)?,
        );
        if packet_size < header_size || header_size < HEADER_SIZE || packet_size > bytes.len() {
            return Err(ParseFailure::InvalidFrame);
        }
        budget.record_frame()?;
        let body = &bytes[header_size..packet_size];
        if operation == OP_MESSAGE {
            match protocol {
                PROTO_ZLIB => {
                    let next_depth = depth.saturating_add(1);
                    budget.record_depth(next_depth)?;
                    let decoded = read_compressed(ZlibDecoder::new(body), budget)?;
                    parse_frames_inner(&decoded, next_depth, budget, messages)?;
                }
                PROTO_BROTLI => {
                    let next_depth = depth.saturating_add(1);
                    budget.record_depth(next_depth)?;
                    let decoded = read_compressed(Decompressor::new(body, 4096), budget)?;
                    parse_frames_inner(&decoded, next_depth, budget, messages)?;
                }
                PROTO_RAW | PROTO_HEARTBEAT => {
                    if let Some(message) = parse_chat_json(body, budget)? {
                        messages.push(message);
                    }
                }
                _ => {}
            }
        }
        bytes = &bytes[packet_size..];
    }
    Ok(())
}

fn read_compressed<R: Read>(reader: R, budget: &mut ParseBudget) -> Result<Vec<u8>, ParseFailure> {
    let frame_remaining = budget.limits.max_expanded_bytes_per_frame;
    let total_remaining = budget
        .limits
        .max_total_expanded_bytes
        .saturating_sub(budget.stats.expanded_bytes);
    let limit = frame_remaining.min(total_remaining);
    let mut decoded = Vec::new();
    reader
        .take(limit.saturating_add(1) as u64)
        .read_to_end(&mut decoded)
        .map_err(|_| ParseFailure::Decompression)?;
    if decoded.len() > frame_remaining {
        return Err(ParseFailure::Budget(BudgetLimit::ExpandedFrameBytes));
    }
    if decoded.len() > total_remaining {
        return Err(ParseFailure::Budget(BudgetLimit::ExpandedTotalBytes));
    }
    budget.stats.expanded_bytes += decoded.len();
    Ok(decoded)
}

fn parse_chat_json(
    bytes: &[u8],
    budget: &mut ParseBudget,
) -> Result<Option<ParsedMessage>, ParseFailure> {
    let Some(value) = serde_json::from_slice::<Value>(bytes).ok() else {
        return Ok(None);
    };
    let Some(command) = value.get("cmd").and_then(Value::as_str) else {
        return Ok(None);
    };
    if command != "DANMU_MSG" {
        return Ok(None);
    }
    let Some(info) = value.get("info").and_then(Value::as_array) else {
        return Ok(None);
    };
    let Some(text) = info.get(1).and_then(Value::as_str).map(str::trim) else {
        return Ok(None);
    };
    let Some(sender_name) = info
        .get(2)
        .and_then(|sender| sender.get(1))
        .and_then(Value::as_str)
        .map(str::trim)
    else {
        return Ok(None);
    };
    if text.is_empty() || sender_name.is_empty() {
        return Ok(None);
    }
    if sender_name.len() > budget.limits.max_sender_name_bytes {
        return Err(ParseFailure::Budget(BudgetLimit::SenderNameBytes));
    }
    if text.len() > budget.limits.max_message_text_bytes {
        return Err(ParseFailure::Budget(BudgetLimit::MessageTextBytes));
    }
    budget.record_message()?;
    Ok(Some(ParsedMessage {
        sender_name: sender_name.to_owned(),
        text: text.to_owned(),
    }))
}

fn report_parse_rejection(error: &ParseError) {
    eprintln!(
        "danmaku packet rejected: reason={:?} input_bytes={} expanded_bytes={} max_depth={} frames={} messages={} elapsed_us={}",
        error.failure,
        error.stats.input_bytes,
        error.stats.expanded_bytes,
        error.stats.max_depth,
        error.stats.frames,
        error.stats.messages,
        error.elapsed.as_micros(),
    );
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

    use super::{
        build_packet, parse_frames, parse_frames_with_limits, parse_text_message, BudgetLimit,
        ParseFailure, ParseLimits, OP_MESSAGE, PROTO_BROTLI, PROTO_HEARTBEAT, PROTO_RAW,
        PROTO_ZLIB,
    };

    fn test_limits() -> ParseLimits {
        ParseLimits {
            max_input_bytes: 256 * 1024,
            max_expanded_bytes_per_frame: 64 * 1024,
            max_total_expanded_bytes: 128 * 1024,
            max_nested_depth: 4,
            max_frames: 128,
            max_messages: 64,
            max_sender_name_bytes: 64,
            max_message_text_bytes: 256,
        }
    }

    fn chat_json(sender_name: &str, text: &str) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "cmd": "DANMU_MSG",
            "info": [null, text, [123, sender_name]],
        }))
        .unwrap()
    }

    fn chat_packet(sender_name: &str, text: &str) -> Vec<u8> {
        build_packet(OP_MESSAGE, PROTO_RAW, &chat_json(sender_name, text))
    }

    fn zlib_packet(inner: &[u8]) -> Vec<u8> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(inner).unwrap();
        build_packet(OP_MESSAGE, PROTO_ZLIB, &encoder.finish().unwrap())
    }

    fn brotli_packet(inner: &[u8]) -> Vec<u8> {
        let mut encoded = Vec::new();
        {
            let mut writer = CompressorWriter::new(&mut encoded, 4096, 5, 22);
            writer.write_all(inner).unwrap();
        }
        build_packet(OP_MESSAGE, PROTO_BROTLI, &encoded)
    }

    fn ignored_frame(body_bytes: usize) -> Vec<u8> {
        build_packet(0, PROTO_RAW, &vec![0; body_bytes])
    }

    #[test]
    fn preserves_normal_raw_compressed_text_and_heartbeat_messages() {
        let chat = chat_packet("观众", "你好");
        let mut heartbeat_then_chat = build_packet(3, PROTO_HEARTBEAT, &42u32.to_be_bytes());
        heartbeat_then_chat.extend_from_slice(&chat);

        for packet in [
            chat.clone(),
            zlib_packet(&chat),
            brotli_packet(&chat),
            heartbeat_then_chat,
        ] {
            let messages = parse_frames(&packet).unwrap();
            assert_eq!(messages.len(), 1);
            assert_eq!(messages[0].sender_name, "观众");
            assert_eq!(messages[0].text, "你好");
        }
        assert_eq!(
            parse_text_message(&chat_json("观众", "你好"))
                .unwrap()
                .unwrap()
                .text,
            "你好"
        );
    }

    #[test]
    fn distinguishes_invalid_frames_from_budget_rejections() {
        let invalid = parse_frames(&[0, 1, 2]).unwrap_err();
        assert_eq!(invalid.failure, ParseFailure::InvalidFrame);
        assert!(!invalid.is_budget_exceeded());

        let packet = chat_packet("观众", "你好");
        let mut limits = test_limits();
        limits.max_input_bytes = packet.len() - 1;
        let error = parse_frames_with_limits(&packet, limits).unwrap_err();

        assert_eq!(error.failure, ParseFailure::Budget(BudgetLimit::InputBytes));
        assert_eq!(error.stats.frames, 0);
        assert!(error.is_budget_exceeded());
    }

    #[test]
    fn enforces_nested_depth_across_zlib_and_brotli() {
        let chat = chat_packet("观众", "你好");
        let depth_two = brotli_packet(&zlib_packet(&chat));
        let mut limits = test_limits();
        limits.max_nested_depth = 2;
        assert_eq!(
            parse_frames_with_limits(&depth_two, limits).unwrap().len(),
            1
        );

        let depth_three = zlib_packet(&depth_two);
        let error = parse_frames_with_limits(&depth_three, limits).unwrap_err();

        assert_eq!(
            error.failure,
            ParseFailure::Budget(BudgetLimit::NestedDepth)
        );
    }

    #[test]
    fn shares_total_expansion_budget_across_sibling_frames() {
        let compressed = zlib_packet(&ignored_frame(700));
        let mut packet = compressed.clone();
        packet.extend_from_slice(&compressed);
        let mut limits = test_limits();
        limits.max_expanded_bytes_per_frame = 1024;
        limits.max_total_expanded_bytes = 1000;

        let error = parse_frames_with_limits(&packet, limits).unwrap_err();

        assert_eq!(
            error.failure,
            ParseFailure::Budget(BudgetLimit::ExpandedTotalBytes)
        );
        assert!(error.stats.expanded_bytes <= limits.max_total_expanded_bytes);
    }

    #[test]
    fn stops_zlib_and_brotli_bombs_at_the_streaming_frame_limit() {
        let expanded = ignored_frame(32 * 1024);
        let mut limits = test_limits();
        limits.max_expanded_bytes_per_frame = 1024;
        limits.max_total_expanded_bytes = 8 * 1024;

        for packet in [zlib_packet(&expanded), brotli_packet(&expanded)] {
            assert!(packet.len() < expanded.len());
            let error = parse_frames_with_limits(&packet, limits).unwrap_err();
            assert_eq!(
                error.failure,
                ParseFailure::Budget(BudgetLimit::ExpandedFrameBytes)
            );
            assert!(error.stats.expanded_bytes <= limits.max_expanded_bytes_per_frame);
        }
    }

    #[test]
    fn bounds_frame_and_compressed_message_floods() {
        let frame = build_packet(3, PROTO_HEARTBEAT, &[]);
        let mut frames = frame.repeat(3);
        let mut limits = test_limits();
        limits.max_frames = 3;
        assert!(parse_frames_with_limits(&frames, limits)
            .unwrap()
            .is_empty());
        frames.extend_from_slice(&frame);
        assert_eq!(
            parse_frames_with_limits(&frames, limits)
                .unwrap_err()
                .failure,
            ParseFailure::Budget(BudgetLimit::Frames)
        );

        let chat = chat_packet("观众", "你好");
        let mut messages = chat.repeat(3);
        let mut limits = test_limits();
        limits.max_messages = 3;
        assert_eq!(
            parse_frames_with_limits(&zlib_packet(&messages), limits)
                .unwrap()
                .len(),
            3
        );
        messages.extend_from_slice(&chat);
        assert_eq!(
            parse_frames_with_limits(&zlib_packet(&messages), limits)
                .unwrap_err()
                .failure,
            ParseFailure::Budget(BudgetLimit::Messages)
        );
    }

    #[test]
    fn enforces_utf8_field_limits_without_exposing_values() {
        let mut limits = test_limits();
        limits.max_sender_name_bytes = "观众".len();
        let rejected_name = "观众甲";
        let name_error =
            parse_frames_with_limits(&chat_packet(rejected_name, "你好"), limits).unwrap_err();
        limits.max_message_text_bytes = "你好".len();
        let text_error =
            parse_frames_with_limits(&chat_packet("观众", "你好啊"), limits).unwrap_err();

        assert_eq!(
            name_error.failure,
            ParseFailure::Budget(BudgetLimit::SenderNameBytes)
        );
        assert_eq!(
            text_error.failure,
            ParseFailure::Budget(BudgetLimit::MessageTextBytes)
        );
        assert!(!format!("{name_error:?}").contains(rejected_name));

        let long_name = "a".repeat(super::DANMAKU_PARSE_LIMITS.max_sender_name_bytes + 1);
        assert_eq!(
            parse_text_message(&chat_json(&long_name, "你好"))
                .unwrap_err()
                .failure,
            ParseFailure::Budget(BudgetLimit::SenderNameBytes)
        );
    }
}
