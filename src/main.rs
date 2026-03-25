//! VibingIDE — Entry Point (egui/eframe GUI)

use anyhow::Result;
use std::sync::Arc;
use tracing::info;

mod app;
mod config;
mod engine;
mod history;
mod path_utils;
mod pty;
mod ui;

fn main() -> Result<()> {
    // ── Logging ──────────────────────────────────────────────────────────────
    let log_dir = dirs::home_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join(".vibingide");
    std::fs::create_dir_all(&log_dir)?;

    let file_appender = tracing_appender::rolling::never(&log_dir, "debug.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_env_filter(
            std::env::var("VIBINGIDE_LOG").unwrap_or_else(|_| "vibingide=info".into()),
        )
        .init();

    info!("VibingIDE starting (GUI mode)");

    // ── Parse CLI args ───────────────────────────────────────────────────────
    let args = parse_args();

    let project_root = args
        .project
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().expect("cannot get cwd"));
    let project_root = project_root
        .canonicalize()
        .map(crate::path_utils::normalize_platform_path)
        .map_err(|e| anyhow::anyhow!("cannot open project directory: {e}"))?;

    // ── Config ───────────────────────────────────────────────────────────────
    let config = config::AppConfig::load(&project_root)?;

    // ── Tokio runtime (background thread pool) ───────────────────────────────
    let rt = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?,
    );

    // ── Launch egui window ───────────────────────────────────────────────────
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("VibingIDE")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 500.0])
            .with_decorations(false)
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "VibingIDE",
        native_options,
        Box::new(move |cc| {
            // Style the egui context
            app::VibingApp::setup_visuals(&cc.egui_ctx);
            Box::new(app::VibingApp::new(
                cc,
                project_root,
                config,
                args.initial_cmd,
                rt,
            ))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {e}"))
}

fn load_icon() -> egui::IconData {
    // Minimal 1×1 icon — replace with real icon bytes if desired.
    egui::IconData {
        rgba:   vec![120, 80, 255, 255],
        width:  1,
        height: 1,
    }
}

// ── CLI arg parsing ───────────────────────────────────────────────────────────

struct CliArgs {
    project:     Option<String>,
    initial_cmd: Option<String>,
}

fn parse_args() -> CliArgs {
    let mut args        = std::env::args().skip(1);
    let mut project     = None::<String>;
    let mut initial_cmd = None::<String>;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--project" | "-p" => { project     = args.next(); }
            "--cmd"     | "-c" => { initial_cmd = args.next(); }
            "--help"    | "-h" => {
                eprintln!(
                    "VibingIDE — Agent-First IDE\n\n\
                     Usage: vibingide [OPTIONS]\n\n\
                     OPTIONS:\n  \
                       -p, --project <DIR>   Project directory to open\n  \
                       -c, --cmd <COMMAND>   Initial agent command to launch\n  \
                       -h, --help            Show this help"
                );
                std::process::exit(0);
            }
            "--version" | "-V" => {
                eprintln!("vibingide {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            other => {
                if !other.starts_with('-') && project.is_none() {
                    project = Some(other.to_string());
                }
            }
        }
    }

    CliArgs { project, initial_cmd }
}
