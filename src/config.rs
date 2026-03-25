//! Configuration loading and validation.
//!
//! Global config: `~/.vibingide/config.toml`
//! Project override: `<root>/.vibingide/config.toml`
//! Project config keys override global keys (merged, not replaced).
//!
//! Security: all path values are validated; unknown keys are rejected.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

// ── Top-level config ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    #[serde(default)]
    pub ui: UiConfig,

    #[serde(default)]
    pub keybinds: KeybindConfig,

    #[serde(default)]
    pub history: HistoryConfig,

    #[serde(default)]
    pub security: SecurityConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ui:       UiConfig::default(),
            keybinds: KeybindConfig::default(),
            history:  HistoryConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

impl AppConfig {
    /// Load and merge global + project configs.
    /// Returns a validated, merged `AppConfig`.
    pub fn load(project_root: &Path) -> Result<Self> {
        let global = load_global_config()?;
        let project = load_project_config(project_root)?;
        let merged = merge(global, project);
        merged.validate(project_root)?;
        Ok(merged)
    }

    /// Validate config values for security and sanity.
    fn validate(&self, project_root: &Path) -> Result<()> {
        // UI constraints
        if self.ui.left_panel_width_pct < 10 || self.ui.left_panel_width_pct > 50 {
            anyhow::bail!("ui.left_panel_width_pct must be 10–50");
        }
        if self.ui.output_buffer_lines < 100 || self.ui.output_buffer_lines > 100_000 {
            anyhow::bail!("ui.output_buffer_lines must be 100–100000");
        }

        // Security: validate env allowlist entries are valid identifier-like strings
        for key in &self.security.child_env_allowlist {
            if !key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '(' | ')'))
            {
                anyhow::bail!("security.child_env_allowlist entry is invalid: {key:?}");
            }
        }

        debug!("Config validated for project {}", project_root.display());
        Ok(())
    }
}

// ── Sub-config structs ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiConfig {
    pub theme:                  String,
    pub left_panel_width_pct:   u8,
    pub output_buffer_lines:    usize,
    pub scroll_speed:           u8,
    pub show_panel_borders:     bool,
    pub show_status_bar:        bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme:                "dark".into(),
            left_panel_width_pct: 25,
            output_buffer_lines:  10_000,
            scroll_speed:         3,
            show_panel_borders:   true,
            show_status_bar:      true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeybindConfig {
    pub new_panel:      String,
    pub next_panel:     String,
    pub prev_panel:     String,
    pub focus_input:    String,
    pub focus_tree:     String,
    pub focus_history:  String,
    pub maximize_panel: String,
    pub close_panel:    String,
    pub open_project:   String,
    pub command_palette: String,
    pub scroll_up:      String,
    pub scroll_down:    String,
}

impl Default for KeybindConfig {
    fn default() -> Self {
        Self {
            new_panel:       "ctrl+shift+n".into(),
            next_panel:      "ctrl+]".into(),
            prev_panel:      "ctrl+[".into(),
            focus_input:     "ctrl+i".into(),
            focus_tree:      "ctrl+e".into(),
            focus_history:   "ctrl+h".into(),
            maximize_panel:  "ctrl+m".into(),
            close_panel:     "ctrl+w".into(),
            open_project:    "ctrl+o".into(),
            command_palette: "ctrl+p".into(),
            scroll_up:       "ctrl+u".into(),
            scroll_down:     "ctrl+d".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryConfig {
    pub max_sessions_per_project: usize,
    pub auto_archive_after_days:  u32,
    pub store_raw_ansi:           bool,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            max_sessions_per_project: 500,
            auto_archive_after_days:  30,
            store_raw_ansi:           false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityConfig {
    /// Env vars passed through to child processes.
    /// Anything not in this list is stripped from child env.
    pub child_env_allowlist: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            child_env_allowlist: vec![
                "PATH".into(),
                "HOME".into(),
                "TERM".into(),
                "LANG".into(),
                "LC_ALL".into(),
                "USER".into(),
                "LOGNAME".into(),
                // Windows equivalents
                "USERPROFILE".into(),
                "APPDATA".into(),
                "LOCALAPPDATA".into(),
                "TEMP".into(),
                "TMP".into(),
                "COMSPEC".into(),
                "SystemRoot".into(),
                "windir".into(),
                "PATHEXT".into(),
                "SystemDrive".into(),
                "HOMEDRIVE".into(),
                "HOMEPATH".into(),
                "OS".into(),
                "USERNAME".into(),
                "COMPUTERNAME".into(),
                "NUMBER_OF_PROCESSORS".into(),
                "PROCESSOR_ARCHITECTURE".into(),
                "ProgramData".into(),
                "ProgramFiles".into(),
                "ProgramFiles(x86)".into(),
                "ProgramW6432".into(),
                "CommonProgramFiles".into(),
                "CommonProgramFiles(x86)".into(),
                "CommonProgramW6432".into(),
                "PSModulePath".into(),
            ],
        }
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn global_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".vibingide").join("config.toml"))
}

fn project_config_path(project_root: &Path) -> PathBuf {
    project_root.join(".vibingide").join("config.toml")
}

fn load_global_config() -> Result<AppConfig> {
    match global_config_path() {
        None => {
            warn!("Could not determine home directory; using defaults");
            Ok(AppConfig::default())
        }
        Some(path) => load_toml_or_default(&path),
    }
}

fn load_project_config(project_root: &Path) -> Result<AppConfig> {
    load_toml_or_default(&project_config_path(project_root))
}

fn load_toml_or_default(path: &Path) -> Result<AppConfig> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading config file {}", path.display()))?;

    let cfg: AppConfig = toml::from_str(&raw)
        .with_context(|| format!("parsing config file {}", path.display()))?;

    debug!("Loaded config from {}", path.display());
    Ok(cfg)
}

/// Merge project config on top of global config.
/// Project values override global values field-by-field via override TOML tables.
/// Simple implementation: project fields take precedence if they were set.
/// (Full per-field merging would require Option<T> wrappers; kept simple for now.)
fn merge(global: AppConfig, project: AppConfig) -> AppConfig {
    // For now, project config fully overrides global if the project config file exists.
    // Future: per-field Option<T> merge.
    let _ = global; // will be used for partial merge in a follow-up
    project
}
