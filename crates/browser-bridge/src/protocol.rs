use crate::policy::{BrowserKind, CapturePolicy};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
pub const MAX_PENDING: usize = 32;
pub const MAX_RECEIPTS: usize = 256;
pub const RETENTION: Duration = Duration::from_secs(600);
#[derive(Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct RequestId(String);
impl RequestId {
    pub fn parse(text: &str) -> Result<Self, ProtocolError> {
        if text.len() != 36
            || !text.bytes().enumerate().all(|(i, b)| {
                if [8, 13, 18, 23].contains(&i) {
                    b == b'-'
                } else {
                    b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
                }
            })
        {
            return Err(ProtocolError);
        }
        Ok(Self(text.into()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl<'de> Deserialize<'de> for RequestId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::parse(&String::deserialize(d)?)
            .map_err(|_| serde::de::Error::custom("Invalid capture identifier"))
    }
}
impl fmt::Debug for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RequestId(..)")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProtocolError;
impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Invalid browser request")
    }
}
impl std::error::Error for ProtocolError {}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureSource {
    Automatic,
    ContextMenu,
}
#[derive(Clone, PartialEq, Eq)]
pub struct CaptureRequest {
    pub browser: BrowserKind,
    pub forced: bool,
    pub policy_revision: u64,
    pub original_url: Option<String>,
    pub id: RequestId,
    pub url: String,
    pub filename_hint: Option<String>,
    pub source: CaptureSource,
    pub download_id: Option<u64>,
    pub expires_at_ms: u64,
}
impl fmt::Debug for CaptureRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CaptureRequest")
            .field("source", &self.source)
            .finish_non_exhaustive()
    }
}
impl CaptureRequest {
    pub fn deadline(&self) -> Instant {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Instant::now() + Duration::from_millis(self.expires_at_ms.saturating_sub(now).min(120_000))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operation {
    Policy(BrowserKind),
    Ping,
    Activate,
    Capture(CaptureRequest),
    Status { capture_id: RequestId },
    Abort { capture_id: RequestId },
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    v: u8,
    id: RequestId,
    #[serde(flatten)]
    operation: WireOperation,
}
#[derive(Serialize, Deserialize)]
#[serde(
    tag = "op",
    content = "payload",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum WireOperation {
    Policy(PolicyTarget),
    Ping(Empty),
    Activate(Empty),
    Capture(CapturePayload),
    Status(Target),
    Abort(Target),
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyTarget {
    #[serde(default)]
    browser: BrowserKind,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Target {
    capture_id: RequestId,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CapturePayload {
    #[serde(default)]
    browser: BrowserKind,
    #[serde(default)]
    forced: bool,
    #[serde(default)]
    policy_revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    original_url: Option<String>,
    url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    filename_hint: Option<String>,
    source: CaptureSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    download_id: Option<u64>,
    expires_at_ms: u64,
}
impl Operation {
    pub fn decode(bytes: &[u8]) -> Result<(RequestId, Self), ProtocolError> {
        // Validate the top level separately: flatten + deny_unknown_fields is unsupported by serde.
        let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| ProtocolError)?;
        let object = value.as_object().ok_or(ProtocolError)?;
        if object.len() != 4
            || !["v", "id", "op", "payload"]
                .iter()
                .all(|k| object.contains_key(*k))
        {
            return Err(ProtocolError);
        }
        let v = object["v"].as_u64().ok_or(ProtocolError)?;
        if v != 1 {
            return Err(ProtocolError);
        }
        let id = RequestId::parse(object["id"].as_str().ok_or(ProtocolError)?)?;
        let wire: WireOperation = serde_json::from_value(
            serde_json::json!({"op":object["op"],"payload":object["payload"]}),
        )
        .map_err(|_| ProtocolError)?;
        let operation = match wire {
            WireOperation::Ping(_) => Self::Ping,
            WireOperation::Policy(p) => Self::Policy(p.browser),
            WireOperation::Activate(_) => Self::Activate,
            WireOperation::Status(t) => Self::Status {
                capture_id: t.capture_id,
            },
            WireOperation::Abort(t) => Self::Abort {
                capture_id: t.capture_id,
            },
            WireOperation::Capture(p) => {
                if p.url.len() > 8192
                    || p.url.chars().any(char::is_control)
                    || p.filename_hint.as_ref().is_some_and(|h| {
                        h.encode_utf16().count() > 255 || h.chars().any(char::is_control)
                    })
                    || (p.source == CaptureSource::Automatic && p.download_id.is_none())
                {
                    return Err(ProtocolError);
                }
                let url = url::Url::parse(&p.url).map_err(|_| ProtocolError)?;
                if !matches!(url.scheme(), "http" | "https")
                    || url.host_str().is_none()
                    || !url.username().is_empty()
                    || url.password().is_some()
                {
                    return Err(ProtocolError);
                }
                if let Some(original) = &p.original_url {
                    let url = url::Url::parse(original).map_err(|_| ProtocolError)?;
                    if original.len() > 8192
                        || original.chars().any(char::is_control)
                        || !matches!(url.scheme(), "http" | "https")
                        || !url.username().is_empty()
                        || url.password().is_some()
                    {
                        return Err(ProtocolError);
                    }
                }
                Self::Capture(CaptureRequest {
                    browser: p.browser,
                    forced: p.forced,
                    policy_revision: p.policy_revision,
                    original_url: p.original_url,
                    id: id.clone(),
                    url: p.url,
                    filename_hint: p.filename_hint,
                    source: p.source,
                    download_id: p.download_id,
                    expires_at_ms: p.expires_at_ms,
                })
            }
        };
        Ok((id, operation))
    }
    pub fn encode(&self, id: RequestId) -> Result<Vec<u8>, ProtocolError> {
        let operation = match self {
            Self::Ping => WireOperation::Ping(Empty {}),
            Self::Policy(browser) => WireOperation::Policy(PolicyTarget { browser: *browser }),
            Self::Activate => WireOperation::Activate(Empty {}),
            Self::Status { capture_id } => WireOperation::Status(Target {
                capture_id: capture_id.clone(),
            }),
            Self::Abort { capture_id } => WireOperation::Abort(Target {
                capture_id: capture_id.clone(),
            }),
            Self::Capture(p) => WireOperation::Capture(CapturePayload {
                browser: p.browser,
                forced: p.forced,
                policy_revision: p.policy_revision,
                original_url: p.original_url.clone(),
                url: p.url.clone(),
                filename_hint: p.filename_hint.clone(),
                source: p.source,
                download_id: p.download_id,
                expires_at_ms: p.expires_at_ms,
            }),
        };
        serde_json::to_vec(&Envelope {
            v: 1,
            id,
            operation,
        })
        .map_err(|_| ProtocolError)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeclineCode {
    InvalidRequest,
    Unsupported,
    Busy,
    UserCancelled,
    BrowserFallback,
    Expired,
    AppClosing,
    EngineUnavailable,
    RecoveryRequired,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReplyStatus {
    Policy {
        browser: BrowserKind,
        policy: CapturePolicy,
    },
    Ok {
        host_version: String,
        gui_running: bool,
    },
    Pending,
    Accepted {
        job_id: u64,
        created_new: bool,
    },
    Declined {
        code: DeclineCode,
    },
    Unknown,
    Aborted,
    AlreadyAccepted {
        job_id: u64,
        created_new: bool,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reply {
    pub v: u8,
    pub id: RequestId,
    #[serde(flatten)]
    pub result: ReplyStatus,
}
impl Reply {
    pub fn new(id: RequestId, result: ReplyStatus) -> Self {
        Self { v: 1, id, result }
    }
}
