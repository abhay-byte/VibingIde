//! History event types serialized to NDJSON session files.

use chrono::Utc;
use serde::{Deserialize, Serialize};

/// A single event in a session's NDJSON history file.
/// `deny_unknown_fields` prevents maliciously crafted NDJSON from
/// adding unexpected variant data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum HistoryEvent {
    SessionStart {
        ts:        i64,
        agent_cmd: String,
        label:     String,
        /// Absolute project path at session creation time.
        cwd:       String,
    },
    UserInput {
        ts:   i64,
        /// Raw text the user typed; newlines stripped.
        text: String,
    },
    AgentOutput {
        ts:   i64,
        /// Plain-text (ANSI stripped) output from the agent.
        text: String,
    },
    SessionEnd {
        ts:        i64,
        exit_code: Option<i32>,
        signal:    Option<i32>,
    },
}

impl HistoryEvent {
    pub fn now_ms() -> i64 {
        Utc::now().timestamp_millis()
    }

    pub fn session_start(agent_cmd: &str, label: &str, cwd: &str) -> Self {
        Self::SessionStart {
            ts:        Self::now_ms(),
            agent_cmd: agent_cmd.to_string(),
            label:     label.to_string(),
            cwd:       cwd.to_string(),
        }
    }

    pub fn user_input(text: &str) -> Self {
        // Strip newlines — the \n is implied by Enter and not part of the message.
        Self::UserInput {
            ts:   Self::now_ms(),
            text: text.replace('\n', "").replace('\r', ""),
        }
    }

    pub fn agent_output(text: &str) -> Self {
        Self::AgentOutput {
            ts:   Self::now_ms(),
            text: text.to_string(),
        }
    }

    pub fn session_end(exit_code: Option<i32>, signal: Option<i32>) -> Self {
        Self::SessionEnd {
            ts: Self::now_ms(),
            exit_code,
            signal,
        }
    }
}
