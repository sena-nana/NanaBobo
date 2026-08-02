use std::time::{SystemTime, UNIX_EPOCH};

use qrcode::{render::svg, QrCode};
use reqwest::{header, Client};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use thiserror::Error;
use url::Url;

use crate::models::{AccountSummary, RoomInfo};

const QR_GENERATE_URL: &str = "https://passport.bilibili.com/x/passport-login/web/qrcode/generate";
const QR_POLL_URL: &str = "https://passport.bilibili.com/x/passport-login/web/qrcode/poll";
const NAV_URL: &str = "https://api.bilibili.com/x/web-interface/nav";
const ROOM_INFO_URL: &str = "https://api.live.bilibili.com/room/v1/Room/get_info";

#[derive(Debug, Error)]
pub enum BilibiliError {
    #[error("upstream request failed")]
    Request(#[source] reqwest::Error),
    #[error("upstream returned HTTP status {0}")]
    HttpStatus(u16),
    #[error("upstream returned an application error")]
    Api { code: i64, message: String },
    #[error("upstream response was invalid")]
    InvalidResponse,
    #[error("login result did not contain a valid session")]
    InvalidLoginUrl,
    #[error("qr code generation failed")]
    QrCode,
}

#[derive(Clone)]
pub struct BilibiliClient {
    http: Client,
}

impl BilibiliClient {
    pub fn new() -> Result<Self, BilibiliError> {
        let mut default_headers = header::HeaderMap::new();
        default_headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static("NanaBobo/0.1.0"),
        );
        Client::builder()
            .default_headers(default_headers)
            .build()
            .map(|http| Self { http })
            .map_err(BilibiliError::Request)
    }

    pub async fn generate_qr(&self) -> Result<(QrSession, String), BilibiliError> {
        let response = self
            .http
            .get(QR_GENERATE_URL)
            .send()
            .await
            .map_err(BilibiliError::Request)?;
        let envelope: ApiEnvelope<QrGenerateData> = parse_json(response).await?;
        let data = envelope.into_data()?;
        let qr = QrCode::new(data.url.as_bytes()).map_err(|_| BilibiliError::QrCode)?;
        let svg = qr.render::<svg::Color>().min_dimensions(256, 256).build();
        let expires_at = now_seconds().saturating_add(180);
        Ok((
            QrSession {
                qrcode_key: data.qrcode_key,
                expires_at,
            },
            svg,
        ))
    }

    pub async fn poll_qr(&self, qrcode_key: &str) -> Result<QrPollResult, BilibiliError> {
        let response = self
            .http
            .get(QR_POLL_URL)
            .query(&[("qrcode_key", qrcode_key)])
            .send()
            .await
            .map_err(BilibiliError::Request)?;
        let response_cookie = cookie_from_set_cookie_headers(response.headers());
        let envelope: ApiEnvelope<QrPollData> = parse_json(response).await?;
        let data = envelope.into_data()?;
        match data.code {
            0 => Ok(QrPollResult::Success {
                cookie: response_cookie
                    .or_else(|| cookie_from_login_url(data.url.as_deref()).ok())
                    .ok_or(BilibiliError::InvalidLoginUrl)?,
            }),
            code => qr_poll_state(code).map_or_else(
                || {
                    Err(BilibiliError::Api {
                        code,
                        message: data.message,
                    })
                },
                Ok,
            ),
        }
    }

    pub async fn account_status(
        &self,
        cookie: &str,
    ) -> Result<Option<AccountSummary>, BilibiliError> {
        let response = self
            .http
            .get(NAV_URL)
            .header(header::COOKIE, cookie)
            .send()
            .await
            .map_err(BilibiliError::Request)?;
        let envelope: ApiEnvelope<NavData> = parse_json(response).await?;
        let data = envelope.into_data()?;
        if !data.is_login {
            return Ok(None);
        }
        Ok(Some(AccountSummary {
            mid: data.mid,
            username: data.uname,
            avatar_url: non_empty(data.face),
        }))
    }

    pub async fn room_info(&self, room_id: u64) -> Result<RoomInfo, BilibiliError> {
        let response = self
            .http
            .get(ROOM_INFO_URL)
            .query(&[("id", room_id)])
            .send()
            .await
            .map_err(BilibiliError::Request)?;
        let envelope: ApiEnvelope<RoomData> = parse_json(response).await?;
        let data = envelope.into_data()?;
        Ok(RoomInfo {
            room_id: data.room_id,
            owner_id: data.uid,
            owner_name: non_empty(data.uname),
            title: data.title,
            live_status: match data.live_status {
                1 => "live".to_owned(),
                2 => "round".to_owned(),
                _ => "offline".to_owned(),
            },
            viewer_count: data.online,
            cover_url: first_non_empty([data.room_cover, data.user_cover, data.keyframe]),
            fetched_at: now_seconds(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct QrSession {
    pub qrcode_key: String,
    pub expires_at: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum QrPollResult {
    Pending,
    Scanned,
    Expired,
    Success { cookie: String },
}

#[derive(Debug, Deserialize)]
struct ApiEnvelope<T> {
    code: i64,
    message: String,
    data: Option<T>,
}

impl<T> ApiEnvelope<T> {
    fn into_data(self) -> Result<T, BilibiliError> {
        if self.code != 0 {
            return Err(BilibiliError::Api {
                code: self.code,
                message: self.message,
            });
        }
        self.data.ok_or(BilibiliError::InvalidResponse)
    }
}

#[derive(Debug, Deserialize)]
struct QrGenerateData {
    url: String,
    qrcode_key: String,
}

#[derive(Debug, Deserialize)]
struct QrPollData {
    code: i64,
    message: String,
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NavData {
    #[serde(rename = "isLogin")]
    is_login: bool,
    mid: u64,
    uname: String,
    face: String,
}

#[derive(Debug, Deserialize)]
struct RoomData {
    room_id: u64,
    uid: u64,
    #[serde(default)]
    uname: String,
    title: String,
    live_status: u8,
    #[serde(default)]
    online: u64,
    #[serde(default)]
    room_cover: String,
    #[serde(default)]
    user_cover: String,
    #[serde(default)]
    keyframe: String,
}

async fn parse_json<T: DeserializeOwned>(response: reqwest::Response) -> Result<T, BilibiliError> {
    let status = response.status();
    if !status.is_success() {
        return Err(BilibiliError::HttpStatus(status.as_u16()));
    }
    response
        .json()
        .await
        .map_err(|_| BilibiliError::InvalidResponse)
}

fn cookie_from_login_url(raw_url: Option<&str>) -> Result<String, BilibiliError> {
    let url = Url::parse(raw_url.ok_or(BilibiliError::InvalidLoginUrl)?)
        .map_err(|_| BilibiliError::InvalidLoginUrl)?;
    let allowed = [
        "SESSDATA",
        "bili_jct",
        "DedeUserID",
        "DedeUserID__ckMd5",
        "sid",
        "Expires",
    ];
    let cookies = url
        .query_pairs()
        .filter(|(key, value)| {
            allowed.contains(&key.as_ref())
                && !value.is_empty()
                && !value.contains(';')
                && !value.contains(['\r', '\n'])
        })
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>();
    if !cookies.iter().any(|cookie| cookie.starts_with("SESSDATA=")) {
        return Err(BilibiliError::InvalidLoginUrl);
    }
    Ok(cookies.join("; "))
}

fn cookie_from_set_cookie_headers(headers: &header::HeaderMap) -> Option<String> {
    let allowed = [
        "SESSDATA",
        "bili_jct",
        "DedeUserID",
        "DedeUserID__ckMd5",
        "sid",
    ];
    let cookies = headers
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .filter_map(|value| value.split(';').next())
        .filter_map(|pair| pair.split_once('='))
        .filter(|(key, value)| {
            allowed.contains(key) && !value.is_empty() && !value.contains(['\r', '\n'])
        })
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>();
    cookies
        .iter()
        .any(|cookie| cookie.starts_with("SESSDATA="))
        .then(|| cookies.join("; "))
}

fn qr_poll_state(code: i64) -> Option<QrPollResult> {
    match code {
        86038 => Some(QrPollResult::Expired),
        86090 => Some(QrPollResult::Scanned),
        86101 => Some(QrPollResult::Pending),
        _ => None,
    }
}

fn non_empty(value: String) -> Option<String> {
    (!value.trim().is_empty()).then_some(value)
}

fn first_non_empty<const N: usize>(values: [String; N]) -> Option<String> {
    values.into_iter().find(|value| !value.trim().is_empty())
}

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use reqwest::header::{HeaderMap, HeaderValue, SET_COOKIE};

    use super::{
        cookie_from_login_url, cookie_from_set_cookie_headers, qr_poll_state, QrPollResult,
    };

    #[test]
    fn login_url_is_reduced_to_supported_cookie_fields() {
        let cookie = cookie_from_login_url(Some(
            "https://passport.bilibili.com/success?SESSDATA=secret%2Cvalue&bili_jct=csrf&unexpected=no",
        ))
        .unwrap();
        assert_eq!(cookie, "SESSDATA=secret,value; bili_jct=csrf");
        assert!(!cookie.contains("unexpected"));
    }

    #[test]
    fn login_url_without_session_is_rejected() {
        assert!(cookie_from_login_url(Some("https://example.com/success?bili_jct=csrf")).is_err());
    }

    #[test]
    fn set_cookie_headers_are_reduced_to_supported_cookie_fields() {
        let mut headers = HeaderMap::new();
        headers.append(
            SET_COOKIE,
            HeaderValue::from_static("SESSDATA=session; Path=/; HttpOnly"),
        );
        headers.append(
            SET_COOKIE,
            HeaderValue::from_static("bili_jct=csrf; Path=/"),
        );
        headers.append(
            SET_COOKIE,
            HeaderValue::from_static("unknown=ignored; Path=/"),
        );
        assert_eq!(
            cookie_from_set_cookie_headers(&headers).as_deref(),
            Some("SESSDATA=session; bili_jct=csrf")
        );
    }

    #[test]
    fn qr_status_codes_match_bilibili_web_login_states() {
        assert_eq!(qr_poll_state(86101), Some(QrPollResult::Pending));
        assert_eq!(qr_poll_state(86090), Some(QrPollResult::Scanned));
        assert_eq!(qr_poll_state(86038), Some(QrPollResult::Expired));
    }
}
