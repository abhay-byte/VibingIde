//! NDJSON-based session history store.
//!
//! Security:
//! - All file paths are validated against the sessions directory.
//! - Serde uses `deny_unknown_fields` on deserialization.
//! - Session files are written with mode 0600 on Unix.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use super::event::HistoryEvent;

/// Manages read/write to a single session's NDJSON file.
pub struct SessionStore {
    path: PathBuf,
}

impl SessionStore {
    /// Open (or create) an NDJSON file for the given session ID.
    /// The path is restricted to `sessions_dir` to prevent path traversal.
    pub fn open(sessions_dir: &Path, session_id: &str) -> Result<Self> {
        // Validate session_id: only alphanumeric + '-' + '_'
        if !session_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            anyhow::bail!("invalid session_id: {session_id:?}");
        }

        std::fs::create_dir_all(sessions_dir)
            .context("creating sessions directory")?;

        let path = sessions_dir.join(format!("{session_id}.ndjson"));

        // Ensure the resolved path is within sessions_dir (prevent traversal).
        let canonical_sessions = crate::path_utils::canonicalize_normalized(sessions_dir)?;
        // We can't canonicalize a non-existent file, so join and check prefix.
        if !path.starts_with(&canonical_sessions) {
            anyhow::bail!("session path escapes sessions directory");
        }

        // Create with restrictive permissions on Unix.
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            OpenOptions::new()
                .create(true)
                .append(true)
                .mode(0o600)
                .open(&path)
                .context("creating session file")?;
        }
        #[cfg(not(unix))]
        {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .context("creating session file")?;
        }

        debug!("Session store opened: {}", path.display());
        Ok(Self { path })
    }

    /// Append a single event to the NDJSON file.
    pub fn append(&self, event: &HistoryEvent) -> Result<()> {
        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.path)
            .context("opening session file for append")?;

        let line = serde_json::to_string(event).context("serializing history event")?;
        writeln!(file, "{line}").context("writing history event")?;
        Ok(())
    }

    /// Read all events from the NDJSON file.
    pub fn read_all(&self) -> Result<Vec<HistoryEvent>> {
        let file = File::open(&self.path).context("opening session file")?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();

        for (line_no, line) in reader.lines().enumerate() {
            let line = line.context("reading session file line")?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            match serde_json::from_str::<HistoryEvent>(trimmed) {
                Ok(ev) => events.push(ev),
                Err(e) => {
                    // Skip malformed lines but log them — don't crash.
                    warn!(line_no, "Skipping malformed NDJSON line: {e}");
                }
            }
        }

        Ok(events)
    }
}

// ── Session index ─────────────────────────────────────────────────────────────

/// Lightweight session metadata stored in `index.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionMeta {
    pub session_id:    String,
    pub label:         String,
    pub agent_cmd:     String,
    pub started_at:    String,   // ISO 8601
    pub ended_at:      Option<String>,
    pub status:        SessionStatus,
    pub first_input:   Option<String>,
    pub message_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Closed,
    Crashed,
}

/// The full index file structure.
#[derive(Debug, Serialize, Deserialize)]
struct IndexFile {
    version:  u32,
    sessions: Vec<SessionMeta>,
}

/// Load session metadata list from `index.json`.
pub fn load_index(vibingide_dir: &Path) -> Result<Vec<SessionMeta>> {
    let path = vibingide_dir.join("index.json");
    if !path.exists() {
        return Ok(Vec::new());
    }

    let raw = std::fs::read_to_string(&path)
        .context("reading index.json")?;

    let idx: IndexFile = serde_json::from_str(&raw)
        .context("parsing index.json")?;

    if idx.version != 1 {
        anyhow::bail!("unsupported index.json version: {}", idx.version);
    }

    Ok(idx.sessions)
}

/// Save updated session metadata list to `index.json`.
pub fn save_index(vibingide_dir: &Path, sessions: &[SessionMeta]) -> Result<()> {
    std::fs::create_dir_all(vibingide_dir)?;
    let path = vibingide_dir.join("index.json");

    let idx = IndexFile {
        version:  1,
        sessions: sessions.to_vec(),
    };

    let json = serde_json::to_string_pretty(&idx)?;
    std::fs::write(&path, json).context("writing index.json")
}
