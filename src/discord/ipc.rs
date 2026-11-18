// Minimal Discord IPC client: frames are [opcode u32 LE][length u32 LE][JSON].
// Linux/macOS: unix socket discord-ipc-N (also inside Flatpak/Snap sandboxes), Windows: named pipe.

use serde_json::{json, Value};
use std::io::{self, Read, Write};
use std::time::Duration;

const OP_HANDSHAKE: u32 = 0;
const OP_FRAME: u32 = 1;
const OP_CLOSE: u32 = 2;
const OP_PING: u32 = 3;
const OP_PONG: u32 = 4;
/// Discord refuses bigger frames
const MAX_FRAME: usize = 64 * 1024;

trait Stream: Read + Write + Send {}
impl<T: Read + Write + Send> Stream for T {}

pub struct Client {
    stream: Box<dyn Stream>,
    nonce: u64,
}

pub fn encode(op: u32, payload: &Value) -> Vec<u8> {
    let body = payload.to_string();
    let mut out = Vec::with_capacity(8 + body.len());
    out.extend_from_slice(&op.to_le_bytes());
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(body.as_bytes());
    out
}

fn read_frame(r: &mut impl Read) -> io::Result<(u32, Value)> {
    let mut hdr = [0u8; 8];
    r.read_exact(&mut hdr)?;
    let op = u32::from_le_bytes(hdr[0..4].try_into().unwrap());
    let len = u32::from_le_bytes(hdr[4..8].try_into().unwrap()) as usize;
    if len > MAX_FRAME {
        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("frame too big: {len}")));
    }
    let mut body = vec![0u8; len];
    r.read_exact(&mut body)?;
    let v = serde_json::from_slice(&body).unwrap_or(Value::Null);
    Ok((op, v))
}

impl Client {
    /// Connects to the first Discord instance found and performs the handshake.
    pub fn connect(client_id: &str) -> io::Result<Self> {
        let mut last = io::Error::new(io::ErrorKind::NotFound, "Discord IPC socket not found");
        for stream in candidates() {
            match stream {
                Ok(s) => {
                    let mut c = Client { stream: s, nonce: 0 };
                    match c.handshake(client_id) {
                        Ok(()) => return Ok(c),
                        Err(e) => last = e,
                    }
                }
                Err(e) if e.kind() != io::ErrorKind::NotFound => last = e,
                Err(_) => {}
            }
        }
        Err(last)
    }

    fn handshake(&mut self, client_id: &str) -> io::Result<()> {
        self.stream.write_all(&encode(OP_HANDSHAKE, &json!({ "v": 1, "client_id": client_id })))?;
        let resp = self.read_reply()?;
        if resp.get("evt").and_then(Value::as_str) == Some("READY") {
            log::info!(
                "connected to Discord as {}",
                resp.pointer("/data/user/username").and_then(Value::as_str).unwrap_or("?")
            );
            Ok(())
        } else {
            Err(io::Error::other(format!("handshake rejected: {resp}")))
        }
    }

    /// Reads until a reply frame, answering pings on the way.
    fn read_reply(&mut self) -> io::Result<Value> {
        loop {
            let (op, v) = read_frame(&mut self.stream)?;
            match op {
                OP_FRAME => return Ok(v),
                OP_PING => self.stream.write_all(&encode(OP_PONG, &v))?,
                OP_CLOSE => {
                    return Err(io::Error::new(io::ErrorKind::ConnectionAborted, format!("closed by Discord: {v}")))
                }
                _ => {}
            }
        }
    }

    /// `activity: None` clears the presence.
    pub fn set_activity(&mut self, activity: Option<&Value>) -> io::Result<()> {
        self.nonce += 1;
        let payload = json!({
            "cmd": "SET_ACTIVITY",
            "args": { "pid": std::process::id(), "activity": activity },
            "nonce": self.nonce.to_string(),
        });
        self.stream.write_all(&encode(OP_FRAME, &payload))?;
        let resp = self.read_reply()?;
        if resp.get("evt").and_then(Value::as_str) == Some("ERROR") {
            log::warn!("Discord rejected the activity: {}", resp.get("data").unwrap_or(&Value::Null));
        }
        Ok(())
    }

    pub fn close(mut self) {
        let _ = self.stream.write_all(&encode(OP_CLOSE, &json!({})));
    }
}

#[cfg(unix)]
fn candidates() -> impl Iterator<Item = io::Result<Box<dyn Stream>>> {
    use std::os::unix::net::UnixStream;
    use std::path::PathBuf;
    let mut dirs: Vec<PathBuf> = Vec::new();
    for var in ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"] {
        if let Some(d) = std::env::var_os(var).map(PathBuf::from) {
            // native, Flatpak (official and Vesktop), Snap
            for sub in ["", "app/com.discordapp.Discord", "app/dev.vencord.Vesktop", "app/com.discordapp.DiscordCanary", "snap.discord", "snap.discord-canary"] {
                dirs.push(d.join(sub));
            }
        }
    }
    dirs.push(PathBuf::from("/tmp"));
    dirs.dedup();
    dirs.into_iter()
        .flat_map(|d| (0..10).map(move |i| d.join(format!("discord-ipc-{i}"))))
        .filter(|p| p.exists())
        .map(|p| {
            let s = UnixStream::connect(&p)?;
            s.set_read_timeout(Some(Duration::from_secs(5)))?;
            s.set_write_timeout(Some(Duration::from_secs(5)))?;
            log::debug!("Discord socket {}", p.display());
            Ok(Box::new(s) as Box<dyn Stream>)
        })
}

#[cfg(windows)]
fn candidates() -> impl Iterator<Item = io::Result<Box<dyn Stream>>> {
    let _ = Duration::ZERO;
    (0..10).map(|i| {
        let f = std::fs::OpenOptions::new().read(true).write(true).open(format!(r"\\.\pipe\discord-ipc-{i}"))?;
        Ok(Box::new(f) as Box<dyn Stream>)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_roundtrip() {
        let v = json!({ "v": 1, "client_id": "123" });
        let bytes = encode(OP_HANDSHAKE, &v);
        assert_eq!(&bytes[0..4], &[0, 0, 0, 0]);
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize, bytes.len() - 8);
        let (op, back) = read_frame(&mut &bytes[..]).unwrap();
        assert_eq!(op, OP_HANDSHAKE);
        assert_eq!(back, v);
    }

    #[test]
    fn oversized_frame_is_rejected() {
        let mut bytes = vec![1, 0, 0, 0];
        bytes.extend_from_slice(&(10_000_000u32).to_le_bytes());
        assert!(read_frame(&mut &bytes[..]).is_err());
    }
}
