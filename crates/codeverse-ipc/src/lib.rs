use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IpcError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Socket path not found")]
    NoSocket,
}

/// Commands that external tools can send to the compositor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum IpcCommand {
    GetWorkspaces,
    SwitchWorkspace { number: usize },
    GetFocusedWindow,
    CloseWindow,
    ReloadConfig,
    ToggleLauncher,
    Ping,
}

/// Responses the compositor sends back.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IpcResponse {
    Workspaces {
        active: usize,
        count: usize,
    },
    FocusedWindow {
        title: Option<String>,
    },
    Ok,
    Pong,
    Error {
        message: String,
    },
}

/// Returns the default IPC socket path for the compositor instance.
pub fn socket_path() -> PathBuf {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(runtime_dir).join("codeverse-compositor.sock")
}

/// Server-side listener that accepts one-shot JSON commands on a Unix socket.
pub struct IpcServer {
    listener: UnixListener,
    path: PathBuf,
}

impl IpcServer {
    /// Bind to the default IPC socket path. Removes stale socket if present.
    pub fn bind() -> Result<Self, IpcError> {
        let path = socket_path();
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        let listener = UnixListener::bind(&path)?;
        listener.set_nonblocking(true)?;
        tracing::info!("IPC listening on {:?}", path);
        Ok(Self { listener, path })
    }

    /// Try to accept and read one command (non-blocking).
    /// Returns `None` when no client is waiting.
    pub fn try_recv(&self) -> Option<(IpcCommand, UnixStream)> {
        match self.listener.accept() {
            Ok((stream, _addr)) => {
                let mut reader = BufReader::new(stream.try_clone().ok()?);
                let mut line = String::new();
                reader.read_line(&mut line).ok()?;
                let cmd: IpcCommand = serde_json::from_str(line.trim()).ok()?;
                Some((cmd, stream))
            }
            Err(_) => None,
        }
    }

    /// Send a response back to the client and close the connection.
    pub fn respond(mut stream: UnixStream, response: &IpcResponse) -> Result<(), IpcError> {
        let json = serde_json::to_string(response)?;
        stream.write_all(json.as_bytes())?;
        stream.write_all(b"\n")?;
        Ok(())
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Client helper: send a command and receive a response.
pub fn send_command(cmd: &IpcCommand) -> Result<IpcResponse, IpcError> {
    let path = socket_path();
    let mut stream = UnixStream::connect(&path)?;
    let json = serde_json::to_string(cmd)?;
    stream.write_all(json.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let response: IpcResponse = serde_json::from_str(line.trim())?;
    Ok(response)
}
