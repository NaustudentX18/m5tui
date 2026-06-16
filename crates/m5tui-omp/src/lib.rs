//! `m5tui-omp` — OMP JSON-RPC-ish frame codec and session stubs.
//!
//! M4 defines a line-oriented protocol: every frame is one line of the
//! form `KIND JSON-LIKE-BODY`. The body uses a tiny subset of JSON
//! (strings only) so we can stay dependency-free. A real serde_json codec
//! can later implement the same `OmpCodec` trait.

use std::collections::HashMap;

/// A single OMP frame flowing between m5Tui and the orchestrator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OmpFrame {
    Ask {
        id: String,
        text: String,
    },
    Answer {
        id: String,
        text: String,
    },
    ToolCall {
        id: String,
        tool: String,
        args: HashMap<String, String>,
    },
    ToolResult {
        id: String,
        output: String,
    },
    TodoUpdate {
        id: String,
        text: String,
        done: bool,
    },
    Subagent {
        id: String,
        task: String,
    },
    StreamingChunk {
        id: String,
        chunk: String,
        finished: bool,
    },
    Thinking {
        id: String,
        text: String,
    },
    Error {
        id: String,
        message: String,
    },
}

impl OmpFrame {
    pub fn id(&self) -> &str {
        match self {
            Self::Ask { id, .. }
            | Self::Answer { id, .. }
            | Self::ToolCall { id, .. }
            | Self::ToolResult { id, .. }
            | Self::TodoUpdate { id, .. }
            | Self::Subagent { id, .. }
            | Self::StreamingChunk { id, .. }
            | Self::Thinking { id, .. }
            | Self::Error { id, .. } => id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodecError {
    BadLine(String),
    BadBody(String),
    UnknownKind(String),
    MissingField(String),
}

impl std::fmt::Display for CodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadLine(s) => write!(f, "bad line: {s}"),
            Self::BadBody(s) => write!(f, "bad body: {s}"),
            Self::UnknownKind(s) => write!(f, "unknown kind: {s}"),
            Self::MissingField(s) => write!(f, "missing field: {s}"),
        }
    }
}

impl std::error::Error for CodecError {}

/// Frame codec trait.
pub trait OmpCodec: Send + Sync {
    fn encode(&self, frame: &OmpFrame) -> String;
    fn decode(&self, line: &str) -> Result<OmpFrame, CodecError>;
}

/// Tiny line-oriented codec. Supports only string fields.
#[derive(Debug, Default, Clone, Copy)]
pub struct LineCodec;

impl LineCodec {
    pub fn new() -> Self {
        Self
    }
}

impl OmpCodec for LineCodec {
    fn encode(&self, frame: &OmpFrame) -> String {
        fn q(v: &str) -> String {
            // Quote values that contain spaces or quotes.
            if v.contains(' ') || v.contains('\"') || v.is_empty() {
                let escaped = v.replace('\"', "\\\"");
                format!("\"{escaped}\"")
            } else {
                v.to_string()
            }
        }
        match frame {
            OmpFrame::Ask { id, text } => format!("ASK id={} text={}", q(id), q(text)),
            OmpFrame::Answer { id, text } => format!("ANS id={} text={}", q(id), q(text)),
            OmpFrame::ToolCall { id, tool, args } => {
                let mut s = format!("CALL id={} tool={}", q(id), q(tool));
                for (k, v) in args {
                    s.push_str(&format!(" arg_{k}={}", q(v)));
                }
                s
            }
            OmpFrame::ToolResult { id, output } => {
                format!("RESULT id={} output={}", q(id), q(output))
            }
            OmpFrame::TodoUpdate { id, text, done } => {
                format!("TODO id={} text={} done={done}", q(id), q(text))
            }
            OmpFrame::Subagent { id, task } => format!("SUB id={} task={}", q(id), q(task)),
            OmpFrame::StreamingChunk {
                id,
                chunk,
                finished,
            } => {
                format!("CHUNK id={} finished={finished} chunk={}", q(id), q(chunk))
            }
            OmpFrame::Thinking { id, text } => format!("THINK id={} text={}", q(id), q(text)),
            OmpFrame::Error { id, message } => format!("ERR id={} message={}", q(id), q(message)),
        }
    }

    fn decode(&self, line: &str) -> Result<OmpFrame, CodecError> {
        let line = line.trim();
        if line.is_empty() {
            return Err(CodecError::BadLine("empty".to_string()));
        }
        let kind_end = line
            .find(' ')
            .ok_or_else(|| CodecError::BadLine(line.to_string()))?;
        let kind = &line[..kind_end];
        let rest = &line[kind_end + 1..];
        let mut fields: HashMap<String, String> = HashMap::new();
        let mut i = 0;
        let bytes = rest.as_bytes();
        while i < bytes.len() {
            // Skip leading spaces.
            while i < bytes.len() && bytes[i] == b' ' {
                i += 1;
            }
            if i >= bytes.len() {
                break;
            }
            // Find '=' separating key and value.
            let key_start = i;
            while i < bytes.len() && bytes[i] != b'=' {
                i += 1;
            }
            if i >= bytes.len() {
                return Err(CodecError::BadBody(rest[key_start..].to_string()));
            }
            let key = std::str::from_utf8(&bytes[key_start..i])
                .map_err(|_| CodecError::BadBody("bad key bytes".to_string()))?
                .to_string();
            i += 1; // skip '='
                    // Parse value: quoted or unquoted.
            let value = if i < bytes.len() && bytes[i] == b'\"' {
                i += 1; // skip opening quote
                let val_start = i;
                while i < bytes.len() && bytes[i] != b'\"' {
                    i += 1;
                }
                if i >= bytes.len() {
                    return Err(CodecError::BadBody("unterminated quote".to_string()));
                }
                let raw = std::str::from_utf8(&bytes[val_start..i])
                    .map_err(|_| CodecError::BadBody("bad value bytes".to_string()))?;
                let val = raw.replace("\\\"", "\"");
                i += 1; // skip closing quote
                val
            } else {
                let val_start = i;
                while i < bytes.len() && bytes[i] != b' ' {
                    i += 1;
                }
                std::str::from_utf8(&bytes[val_start..i])
                    .map_err(|_| CodecError::BadBody("bad value bytes".to_string()))?
                    .to_string()
            };
            fields.insert(key, value);
        }
        let id = fields
            .get("id")
            .ok_or_else(|| CodecError::MissingField("id".to_string()))?
            .clone();
        match kind {
            "ASK" => Ok(OmpFrame::Ask {
                id,
                text: fields.get("text").cloned().unwrap_or_default(),
            }),
            "ANS" => Ok(OmpFrame::Answer {
                id,
                text: fields.get("text").cloned().unwrap_or_default(),
            }),
            "CALL" => {
                let tool = fields.get("tool").cloned().unwrap_or_default();
                let mut args = HashMap::new();
                for (k, v) in fields {
                    if let Some(name) = k.strip_prefix("arg_") {
                        args.insert(name.to_string(), v);
                    }
                }
                Ok(OmpFrame::ToolCall { id, tool, args })
            }
            "RESULT" => Ok(OmpFrame::ToolResult {
                id,
                output: fields.get("output").cloned().unwrap_or_default(),
            }),
            "TODO" => {
                let done = fields.get("done").map(|s| s == "true").unwrap_or(false);
                Ok(OmpFrame::TodoUpdate {
                    id,
                    text: fields.get("text").cloned().unwrap_or_default(),
                    done,
                })
            }
            "SUB" => Ok(OmpFrame::Subagent {
                id,
                task: fields.get("task").cloned().unwrap_or_default(),
            }),
            "CHUNK" => {
                let finished = fields.get("finished").map(|s| s == "true").unwrap_or(false);
                Ok(OmpFrame::StreamingChunk {
                    id,
                    chunk: fields.get("chunk").cloned().unwrap_or_default(),
                    finished,
                })
            }
            "THINK" => Ok(OmpFrame::Thinking {
                id,
                text: fields.get("text").cloned().unwrap_or_default(),
            }),
            "ERR" => Ok(OmpFrame::Error {
                id,
                message: fields.get("message").cloned().unwrap_or_default(),
            }),
            other => Err(CodecError::UnknownKind(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError {
    NotStarted,
    AlreadyStarted,
    Codec(String),
    Stub(String),
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotStarted => write!(f, "session not started"),
            Self::AlreadyStarted => write!(f, "session already started"),
            Self::Codec(s) => write!(f, "codec: {s}"),
            Self::Stub(s) => write!(f, "stub: {s}"),
        }
    }
}

impl std::error::Error for SessionError {}

/// OMP session abstraction.
pub trait OmpSession: Send + Sync {
    fn start(&mut self) -> Result<(), SessionError>;
    fn stop(&mut self) -> Result<(), SessionError>;
    fn send(&mut self, frame: &OmpFrame) -> Result<(), SessionError>;
    fn recv(&mut self) -> Result<Option<OmpFrame>, SessionError>;
}

/// Stub session that replays a canned sequence of frames.
pub struct StubOmpSession {
    started: bool,
    codec: LineCodec,
    canned: Vec<OmpFrame>,
    position: usize,
    sent: Vec<OmpFrame>,
}

impl StubOmpSession {
    pub fn new(canned: Vec<OmpFrame>) -> Self {
        Self {
            started: false,
            codec: LineCodec::new(),
            canned,
            position: 0,
            sent: Vec::new(),
        }
    }

    pub fn simple() -> Self {
        Self::new(vec![
            OmpFrame::Thinking {
                id: "1".to_string(),
                text: "2+2 is 4".to_string(),
            },
            OmpFrame::Answer {
                id: "1".to_string(),
                text: "4".to_string(),
            },
        ])
    }

    pub fn sent(&self) -> &[OmpFrame] {
        &self.sent
    }
}

impl OmpSession for StubOmpSession {
    fn start(&mut self) -> Result<(), SessionError> {
        if self.started {
            return Err(SessionError::AlreadyStarted);
        }
        self.started = true;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), SessionError> {
        if !self.started {
            return Err(SessionError::NotStarted);
        }
        self.started = false;
        Ok(())
    }

    fn send(&mut self, frame: &OmpFrame) -> Result<(), SessionError> {
        if !self.started {
            return Err(SessionError::NotStarted);
        }
        self.codec.encode(frame);
        self.sent.push(frame.clone());
        Ok(())
    }

    fn recv(&mut self) -> Result<Option<OmpFrame>, SessionError> {
        if !self.started {
            return Err(SessionError::NotStarted);
        }
        if self.position >= self.canned.len() {
            return Ok(None);
        }
        let frame = self.canned[self.position].clone();
        self.position += 1;
        Ok(Some(frame))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codec() -> LineCodec {
        LineCodec::new()
    }

    #[test]
    fn codec_ask_round_trip() {
        let c = codec();
        let f = OmpFrame::Ask {
            id: "1".to_string(),
            text: "what is 2+2?".to_string(),
        };
        let line = c.encode(&f);
        let decoded = c.decode(&line).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(decoded, f);
    }

    #[test]
    fn codec_tool_call_args_round_trip() {
        let c = codec();
        let mut args = HashMap::new();
        args.insert("query".to_string(), "weather".to_string());
        let f = OmpFrame::ToolCall {
            id: "2".to_string(),
            tool: "web_search".to_string(),
            args,
        };
        let line = c.encode(&f);
        let decoded = c.decode(&line).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(decoded, f);
    }

    #[test]
    fn codec_rejects_unknown_kind() {
        let c = codec();
        let r = c.decode("FAKE id=1");
        assert!(matches!(r, Err(CodecError::UnknownKind(_))));
    }

    #[test]
    fn stub_session_replays_canned_frames() {
        let mut s = StubOmpSession::simple();
        s.start().unwrap_or_else(|e| panic!("{e}"));
        let first = s.recv().unwrap_or_else(|e| panic!("{e}"));
        assert!(matches!(first, Some(OmpFrame::Thinking { .. })));
        let second = s.recv().unwrap_or_else(|e| panic!("{e}"));
        assert!(matches!(second, Some(OmpFrame::Answer { .. })));
        assert!(s.recv().unwrap_or_else(|e| panic!("{e}")).is_none());
    }

    #[test]
    fn stub_session_records_sent_frames() {
        let mut s = StubOmpSession::simple();
        s.start().unwrap_or_else(|e| panic!("{e}"));
        let ask = OmpFrame::Ask {
            id: "1".to_string(),
            text: "hello".to_string(),
        };
        s.send(&ask).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(s.sent().len(), 1);
        assert_eq!(s.sent()[0].id(), "1");
    }
}
