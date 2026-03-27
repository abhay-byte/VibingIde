//! PTY process supervisor.
//!
//! Security guarantees:
//! - Commands are spawned with `execvp`-style args (Vec<String>), never via an implicit shell.
//! - Child environment is filtered to an explicit allowlist.
//! - PTY size is validated before passing to the kernel.
//! - No `unsafe` code in this module; all unsafe is inside `portable-pty`.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, PtyPair, PtySize};
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

pub type PanelId = u32;

/// A message produced by the reader task and sent over the output channel.
#[derive(Debug)]
pub enum PtyEvent {
    /// Raw bytes (already ANSI-tagged) from the agent's stdout/stderr.
    Output { panel_id: PanelId, data: Vec<u8> },
    /// Child process exited.
    Exited { panel_id: PanelId, exit_code: Option<i32> },
}

#[derive(Debug)]
struct LaunchSpec {
    program: OsString,
    args: Vec<OsString>,
    display_command: String,
}

/// Owns a single child process + PTY pair.
pub struct Supervisor {
    pub panel_id: PanelId,
    pub command: String,
    pub args: Vec<String>,
    pub pid: Option<u32>,
    pty_pair: PtyPair,
    writer: Box<dyn std::io::Write + Send>,
    child_killer: Box<dyn ChildKiller + Send + Sync>,
}

impl Supervisor {
    /// Spawn a new child process inside a PTY.
    ///
    /// # Security
    /// - `command` must be a bare executable name or absolute path.
    /// - `args` are passed directly without shell interpolation.
    /// - `env_allowlist` is the set of env var names to pass to the child.
    pub fn spawn(
        panel_id: PanelId,
        command: &str,
        args: &[String],
        cwd: &PathBuf,
        env_allowlist: &[String],
        runtime_handle: Handle,
        initial_size: PtySize,
        event_tx: mpsc::UnboundedSender<PtyEvent>,
    ) -> Result<Self> {
        let size = sanitize_pty_size(initial_size);

        let pty_system = native_pty_system();
        let pty_pair = pty_system.openpty(size).context("failed to open PTY")?;

        let launch_spec = build_launch_spec(command, args, cwd);
        let mut cmd = CommandBuilder::new(&launch_spec.program);
        cmd.args(&launch_spec.args);
        cmd.cwd(cwd);
        apply_env_allowlist(&mut cmd, env_allowlist);

        let mut child = pty_pair
            .slave
            .spawn_command(cmd)
            .with_context(|| format!("failed to spawn agent process: {}", launch_spec.display_command))?;

        let pid = child.process_id();
        let child_killer = child.clone_killer();

        info!(
            panel_id,
            command = %launch_spec.display_command,
            pid = pid.unwrap_or(0),
            "Agent process spawned"
        );
        debug!(panel_id, ?cwd, "Spawned in cwd");

        let writer = pty_pair
            .master
            .take_writer()
            .context("failed to get PTY writer")?;
        let mut reader = pty_pair
            .master
            .try_clone_reader()
            .context("failed to clone PTY reader")?;
        let pid_for_task = panel_id;

        runtime_handle.spawn_blocking(move || {
            let mut buf = [0u8; 4096];
            loop {
                match std::io::Read::read(&mut reader, &mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let data = buf[..n].to_vec();
                        if event_tx
                            .send(PtyEvent::Output {
                                panel_id: pid_for_task,
                                data,
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(e) => {
                        warn!(panel_id = pid_for_task, "PTY read error: {e}");
                        break;
                    }
                }
            }

            let exit_code = match child.wait() {
                Ok(status) => Some(status.exit_code() as i32),
                Err(e) => {
                    warn!(panel_id = pid_for_task, "PTY child wait error: {e}");
                    None
                }
            };

            let _ = event_tx.send(PtyEvent::Exited {
                panel_id: pid_for_task,
                exit_code,
            });
        });

        Ok(Self {
            panel_id,
            command: command.to_string(),
            args: args.to_vec(),
            pid,
            pty_pair,
            writer,
            child_killer,
        })
    }

    /// Write a line of text to the agent's stdin.
    pub fn send_input(&mut self, text: &str) -> Result<()> {
        let mut bytes = text.as_bytes().to_vec();
        bytes.extend_from_slice(b"\r\n");
        self.send_bytes(&bytes)
    }

    /// Write raw bytes to the PTY stdin.
    pub fn send_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        use std::io::Write;
        self.writer.write_all(bytes).context("writing to PTY stdin")?;
        self.writer.flush().context("flushing PTY stdin")
    }

    pub fn process_id(&self) -> Option<u32> {
        self.pid
    }

    pub fn kill(&mut self) -> Result<()> {
        self.child_killer.kill().context("terminating child process")
    }

    /// Notify the PTY (and child process) of a terminal resize.
    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        let size = sanitize_pty_size(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
        self.pty_pair.master.resize(size).context("resizing PTY")
    }
}

fn apply_env_allowlist(cmd: &mut CommandBuilder, env_allowlist: &[String]) {
    cmd.env_clear();

    let allowlist_upper: Vec<String> = env_allowlist
        .iter()
        .map(|key| key.to_ascii_uppercase())
        .collect();

    for (key, value) in std::env::vars_os() {
        let allowed = if cfg!(windows) {
            key.to_string_lossy()
                .to_ascii_uppercase()
                .as_str()
                .to_owned()
        } else {
            key.to_string_lossy().to_string()
        };

        let is_allowed = if cfg!(windows) {
            allowlist_upper.contains(&allowed)
        } else {
            env_allowlist.iter().any(|entry| entry == &allowed)
        };

        if is_allowed {
            cmd.env(key, value);
        }
    }
}

fn build_launch_spec(command: &str, args: &[String], cwd: &Path) -> LaunchSpec {
    #[cfg(windows)]
    {
        return build_windows_launch_spec(command, args, cwd);
    }

    #[cfg(not(windows))]
    {
        LaunchSpec {
            program: OsString::from(command),
            args: args.iter().map(OsString::from).collect(),
            display_command: render_command(command, args),
        }
    }
}

#[cfg(windows)]
fn build_windows_launch_spec(command: &str, args: &[String], cwd: &Path) -> LaunchSpec {
    let resolved = resolve_windows_command(command, cwd);
    let extension = resolved
        .extension()
        .and_then(OsStr::to_str)
        .map(|ext| ext.to_ascii_lowercase());

    match extension.as_deref() {
        Some("cmd") | Some("bat") => {
            let mut launch_args = vec![OsString::from("/d"), OsString::from("/c"), resolved.as_os_str().to_owned()];
            launch_args.extend(args.iter().map(OsString::from));
            LaunchSpec {
                program: resolve_comspec(),
                args: launch_args,
                display_command: render_command(&resolved.display().to_string(), args),
            }
        }
        Some("ps1") => {
            let mut launch_args = vec![
                OsString::from("-NoLogo"),
                OsString::from("-NoProfile"),
                OsString::from("-ExecutionPolicy"),
                OsString::from("Bypass"),
                OsString::from("-File"),
                resolved.as_os_str().to_owned(),
            ];
            launch_args.extend(args.iter().map(OsString::from));
            LaunchSpec {
                program: resolve_powershell(),
                args: launch_args,
                display_command: render_command(&resolved.display().to_string(), args),
            }
        }
        _ => LaunchSpec {
            program: resolved.as_os_str().to_owned(),
            args: args.iter().map(OsString::from).collect(),
            display_command: render_command(&resolved.display().to_string(), args),
        },
    }
}

#[cfg(windows)]
fn resolve_windows_command(command: &str, cwd: &Path) -> PathBuf {
    let command_path = Path::new(command);
    if command_path.components().count() > 1 || command_path.is_absolute() {
        return resolve_windows_path_candidate(command_path, cwd);
    }

    let has_extension = command_path.extension().is_some();
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    let path_dirs: Vec<PathBuf> = std::env::split_paths(&path_var).collect();

    for dir in &path_dirs {
        if has_extension {
            let candidate = dir.join(command_path);
            if candidate.exists() {
                return candidate;
            }
            continue;
        }
    }

    if !has_extension {
        for ext in windows_path_exts() {
            for dir in &path_dirs {
                let candidate = dir.join(command_path).with_extension(ext.trim_start_matches('.'));
                if candidate.exists() {
                    return candidate;
                }
            }
        }

        for dir in &path_dirs {
            let candidate = dir.join(command_path);
            if candidate.exists() {
                return candidate;
            }
        }
    }

    if has_extension {
        command_path.to_path_buf()
    } else {
        resolve_windows_path_candidate(command_path, cwd)
    }
}

#[cfg(windows)]
fn resolve_windows_path_candidate(command_path: &Path, cwd: &Path) -> PathBuf {
    let base = if command_path.is_absolute() {
        command_path.to_path_buf()
    } else {
        cwd.join(command_path)
    };

    if command_path.extension().is_some() {
        return base;
    }

    if let Some(candidate) = first_existing_candidate(&base, &windows_path_exts()) {
        return candidate;
    }

    if base.exists() {
        return base;
    }

    command_path.to_path_buf()
}

#[cfg(windows)]
fn first_existing_candidate(base: &Path, extensions: &[String]) -> Option<PathBuf> {
    for ext in extensions {
        let candidate = base.with_extension(ext.trim_start_matches('.'));
        if candidate.exists() {
            return Some(candidate);
        }
    }

    if base.exists() {
        return Some(base.to_path_buf());
    }

    None
}

#[cfg(windows)]
fn windows_path_exts() -> Vec<String> {
    let raw = std::env::var("PATHEXT")
        .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string());
    let mut ordered = vec![
        ".EXE".to_string(),
        ".COM".to_string(),
        ".CMD".to_string(),
        ".BAT".to_string(),
    ];

    for ext in raw.split(';').filter_map(|ext| {
        let trimmed = ext.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_ascii_uppercase())
        }
    }) {
        if !ordered.contains(&ext) {
            ordered.push(ext);
        }
    }

    ordered
}

#[cfg(windows)]
fn resolve_comspec() -> OsString {
    std::env::var_os("ComSpec").unwrap_or_else(|| OsString::from("cmd.exe"))
}

#[cfg(windows)]
fn resolve_powershell() -> OsString {
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path_var) {
        let pwsh = dir.join("pwsh.exe");
        if pwsh.exists() {
            return pwsh.into_os_string();
        }
        let powershell = dir.join("powershell.exe");
        if powershell.exists() {
            return powershell.into_os_string();
        }
    }

    OsString::from("powershell.exe")
}

fn render_command(command: &str, args: &[String]) -> String {
    if args.is_empty() {
        command.to_string()
    } else {
        format!("{} {}", command, args.join(" "))
    }
}

/// Clamp PTY dimensions to sane values to avoid kernel errors.
fn sanitize_pty_size(size: PtySize) -> PtySize {
    PtySize {
        rows: size.rows.clamp(2, 500),
        cols: size.cols.clamp(10, 500),
        pixel_width: 0,
        pixel_height: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    use std::path::PathBuf;

    #[cfg(unix)]
    use tokio::time::{timeout, Duration};

    #[cfg(windows)]
    #[test]
    fn windows_resolution_prefers_real_executables_before_extensionless_shims() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("first");
        let second = temp.path().join("second");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();

        std::fs::write(first.join("codex"), "shim").unwrap();
        std::fs::write(first.join("codex.cmd"), "@echo off\r\n").unwrap();
        std::fs::write(second.join("codex.exe"), b"MZ").unwrap();

        let previous_path = std::env::var_os("PATH");
        let previous_pathext = std::env::var_os("PATHEXT");
        std::env::set_var("PATH", std::env::join_paths([first.as_os_str(), second.as_os_str()]).unwrap());
        std::env::set_var("PATHEXT", ".CMD;.EXE");

        let resolved = resolve_windows_command("codex", temp.path());

        if let Some(path) = previous_path {
            std::env::set_var("PATH", path);
        } else {
            std::env::remove_var("PATH");
        }
        if let Some(pathext) = previous_pathext {
            std::env::set_var("PATHEXT", pathext);
        } else {
            std::env::remove_var("PATHEXT");
        }

        assert_eq!(
            resolved.to_string_lossy().to_ascii_lowercase(),
            second.join("codex.exe").to_string_lossy().to_ascii_lowercase()
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_path_candidate_uses_extension_search_for_relative_commands() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("tool.exe"), b"MZ").unwrap();

        let previous_pathext = std::env::var_os("PATHEXT");
        std::env::set_var("PATHEXT", ".EXE;.CMD");

        let resolved = resolve_windows_path_candidate(Path::new("tool"), temp.path());

        if let Some(pathext) = previous_pathext {
            std::env::set_var("PATHEXT", pathext);
        } else {
            std::env::remove_var("PATHEXT");
        }

        assert_eq!(
            resolved.to_string_lossy().to_ascii_lowercase(),
            temp.path()
                .join("tool.exe")
                .to_string_lossy()
                .to_ascii_lowercase()
        );
    }

    #[cfg(unix)]
    fn test_env_allowlist() -> Vec<String> {
        ["PATH", "TERM", "HOME", "LANG", "LC_ALL"]
            .into_iter()
            .map(String::from)
            .collect()
    }

    #[cfg(unix)]
    fn test_size() -> PtySize {
        PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        }
    }

    #[cfg(unix)]
    fn test_cwd() -> PathBuf {
        std::env::current_dir().expect("current dir")
    }

    #[cfg(unix)]
    async fn wait_for_output(
        rx: &mut mpsc::UnboundedReceiver<PtyEvent>,
        needle: &str,
    ) -> String {
        let mut output = String::new();
        while !output.contains(needle) {
            let event = timeout(Duration::from_secs(10), rx.recv())
                .await
                .expect("timed out waiting for PTY event")
                .expect("PTY channel closed unexpectedly");

            match event {
                PtyEvent::Output { data, .. } => output.push_str(&String::from_utf8_lossy(&data)),
                PtyEvent::Exited { exit_code, .. } => {
                    panic!("process exited before expected output {needle:?}: {exit_code:?}; output: {output:?}");
                }
            }
        }
        output
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "current_thread")]
    async fn send_bytes_round_trips_raw_terminal_sequences() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut supervisor = Supervisor::spawn(
            1,
            "/bin/sh",
            &[
                "-lc".into(),
                "stty raw -echo; dd bs=1 count=3 2>/dev/null | od -An -t x1".into(),
            ],
            &test_cwd(),
            &test_env_allowlist(),
            Handle::current(),
            test_size(),
            tx,
        )
        .expect("spawn raw-byte test child");

        supervisor
            .send_bytes(b"\x1b[A")
            .expect("send raw arrow sequence");

        let output = wait_for_output(&mut rx, "1b 5b 41").await;
        let _ = supervisor.kill();

        assert!(output.contains("1b 5b 41"), "unexpected output: {output:?}");
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "current_thread")]
    async fn ctrl_c_reaches_the_child_process_as_sigint() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut supervisor = Supervisor::spawn(
            2,
            "/bin/sh",
            &[
                "-lc".into(),
                "trap 'printf INT\\n; exit 0' INT; printf READY\\n; while :; do sleep 1; done".into(),
            ],
            &test_cwd(),
            &test_env_allowlist(),
            Handle::current(),
            test_size(),
            tx,
        )
        .expect("spawn sigint test child");

        let ready = wait_for_output(&mut rx, "READY").await;
        assert!(ready.contains("READY"), "unexpected startup output: {ready:?}");

        supervisor.send_bytes(&[0x03]).expect("send ctrl-c");

        let output = wait_for_output(&mut rx, "INT").await;
        let _ = supervisor.kill();

        assert!(output.contains("INT"), "unexpected sigint output: {output:?}");
    }
}
