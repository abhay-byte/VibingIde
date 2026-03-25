//! Stub: session manager — links panel IDs to NDJSON session files.
//! Full implementation is part of Milestone 3.

use std::path::Path;
use anyhow::Result;
use ulid::Ulid;

use crate::history::store::{load_index, SessionMeta};

pub struct SessionManager {
    pub sessions: Vec<SessionMeta>,
    vibide_dir:   std::path::PathBuf,
}

impl SessionManager {
    /// Load existing sessions from the project's .vibingide directory.
    pub fn load(vibide_dir: &Path) -> Result<Self> {
        let sessions = load_index(vibide_dir).unwrap_or_default();
        Ok(Self {
            sessions,
            vibide_dir: vibide_dir.to_path_buf(),
        })
    }

    /// Generate a new ULID-based session ID.
    pub fn new_session_id() -> String {
        Ulid::new().to_string()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self {
            sessions:  Vec::new(),
            vibide_dir: std::path::PathBuf::new(),
        }
    }
}
