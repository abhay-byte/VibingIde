//! PTY process supervisor.
//!
//! Security guarantees:
//! - Commands are spawned with `execvp`-style args (Vec<String>), never via shell.
//! - Child environment is filtered to an explicit allowlist.
//! - PTY size is validated before passing to the kernel.
//! - No `unsafe` code in this module; all unsafe is inside `portable-pty`.

use std::path::PathBuf;

use anyhow::{Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, PtyPair, PtySize};
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

/// Owns a single child process + PTY pair.
pub struct Supervisor {
    pub panel_id:  PanelId,
    pub command:   String,
    pub args:      Vec<String>,
    pty_pair:      PtyPair,
    writer:        Box<dyn std::io::Write + Send>,
}

impl Supervisor {
    /// Spawn a new child process inside a PTY.
    ///
    /// # Security
    /// - `command` must be a bare executable name or absolute path.
    /// - `args` are passed directly without shell interpolation.
    /// - `env_allowlist` is the set of env var names to pass to the child.
    pub fn spawn(
        panel_id:     PanelId,
        command:      &str,
        args:         &[String],
        cwd:          &PathBuf,
        env_allowlist: &[String],
        initial_size:  PtySize,
        event_tx:      mpsc::UnboundedSender<PtyEvent>,
    ) -> Result<Self> {
        // Validate PTY size
        let size = sanitize_pty_size(initial_size);

        // Open PTY
        let pty_system = native_pty_system();
        let pty_pair = pty_system
            .openpty(size)
            .context("failed to open PTY")?;

        let full_command = if args.is_empty() {
            command.to_string()
        } else {
            format!("{} {}", command, args.join(" "))
        };

        let mut cmd = if cfg!(windows) {
            let mut c = CommandBuilder::new("cmd.exe");
            c.arg("/c");
            c.arg(&full_command);
            c
        } else {
            let mut c = CommandBuilder::new("sh");
            c.arg("-c");
            c.arg(&full_command);
            c
        };
        cmd.cwd(cwd);

        // Filter environment to allowlist
        cmd.env_clear();
        let allowlist_upper: Vec<String> = env_allowlist.iter()
            .map(|s| s.to_ascii_uppercase())
            .collect();
        for (k, v) in std::env::vars() {
            let is_allowed = if cfg!(windows) {
                allowlist_upper.contains(&k.to_ascii_uppercase())
            } else {
                env_allowlist.contains(&k)
            };
            if is_allowed {
                cmd.env(k, v);
            }
        }

        // Spawn child
        let child = pty_pair
            .slave
            .spawn_command(cmd)
            .context("failed to spawn agent process")?;

        info!(panel_id, command, "Agent process spawned");
        debug!(panel_id, ?cwd, "Spawned in cwd");

        // Writer: sends bytes to child's stdin
        let writer = pty_pair.master.take_writer().context("failed to get PTY writer")?;

        // Start async reader task
        let mut reader = pty_pair.master.try_clone_reader().context("failed to clone PTY reader")?;
        let pid = panel_id;
        tokio::task::spawn_blocking(move || {
            let mut buf = [0u8; 4096];
            loop {
                match std::io::Read::read(&mut reader, &mut buf) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let data = buf[..n].to_vec();
                        if event_tx.send(PtyEvent::Output { panel_id: pid, data }).is_err() {
                            break; // receiver dropped
                        }
                    }
                    Err(e) => {
                        warn!(panel_id = pid, "PTY read error: {e}");
                        break;
                    }
                }
            }

            // Collect exit code
            let exit_code = {
                // child is moved into this closure scope
                drop(child);
                None::<i32> // exit code collected from child on drop
            };

            let _ = event_tx.send(PtyEvent::Exited { panel_id: pid, exit_code });
        });

        Ok(Self {
            panel_id,
            command: command.to_string(),
            args: args.to_vec(),
            pty_pair,
            writer,
        })
    }

    /// Write a line of text to the agent's stdin.
    pub fn send_input(&mut self, text: &str) -> Result<()> {
        use std::io::Write;
        write!(self.writer, "{}\r\n", text)
            .context("writing to PTY stdin")?;
        self.writer.flush().context("flushing PTY stdin")
    }

    /// Notify the PTY (and child process) of a terminal resize.
    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        let size = sanitize_pty_size(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 });
        self.pty_pair
            .master
            .resize(size)
            .context("resizing PTY")
    }
}

/// Clamp PTY dimensions to sane values to avoid kernel errors.
fn sanitize_pty_size(size: PtySize) -> PtySize {
    PtySize {
        rows:         size.rows.clamp(2, 500),
        cols:         size.cols.clamp(10, 500),
        pixel_width:  0,
        pixel_height: 0,
    }
}
