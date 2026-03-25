//! Project directory scanner.
//!
//! Reads a project root and produces a file tree, respecting .gitignore rules.
//! No file content is read — only paths and metadata.

use std::path::{Path, PathBuf};

use anyhow::Result;
use tracing::debug;

// ── File tree types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct FileNode {
    pub name:     String,
    pub path:     PathBuf,
    pub kind:     FileKind,
    pub children: Vec<FileNode>, // empty if kind == File
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileKind {
    File,
    Directory,
}

impl FileNode {
    pub fn is_dir(&self) -> bool {
        self.kind == FileKind::Directory
    }
}

// ── Project ───────────────────────────────────────────────────────────────────

pub struct Project {
    pub root:       PathBuf,
    pub name:       String,
    pub file_tree:  Vec<FileNode>,
    pub vibide_dir: PathBuf, // <root>/.vibingide/
}

impl Project {
    /// Open a project directory.
    /// The root must already be canonicalized by the caller.
    pub fn open(root: PathBuf) -> Result<Self> {
        let name = root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "project".to_string());

        let vibide_dir = root.join(".vibingide");
        std::fs::create_dir_all(&vibide_dir)?;
        std::fs::create_dir_all(vibide_dir.join("sessions"))?;

        let file_tree = scan_directory(&root, &root, 0)?;
        debug!("Scanned {} top-level entries", file_tree.len());

        Ok(Self { root, name, file_tree, vibide_dir })
    }

    /// Re-scan the project directory (call after file system events).
    pub fn refresh(&mut self) -> Result<()> {
        self.file_tree = scan_directory(&self.root, &self.root, 0)?;
        Ok(())
    }
}

// ── Scanner ───────────────────────────────────────────────────────────────────

const MAX_DEPTH: usize = 8;
const MAX_ENTRIES_PER_DIR: usize = 500;

/// Recursively scan a directory, skipping hidden dirs and `.vibingide`.
fn scan_directory(root: &Path, dir: &Path, depth: usize) -> Result<Vec<FileNode>> {
    if depth > MAX_DEPTH {
        return Ok(vec![]);
    }

    let gitignore = load_gitignore_patterns(dir);

    let mut entries = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            // Skip hidden directories (Unix .xxx)
            if name.starts_with('.') && e.path().is_dir() {
                return false;
            }
            // Skip .vibingide entirely
            if name == ".vibingide" {
                return false;
            }
            // Skip gitignored entries
            if is_ignored(&name, &gitignore) {
                return false;
            }
            true
        })
        .collect::<Vec<_>>();

    // Sort: directories first, then files, both alphabetical
    entries.sort_by(|a, b| {
        let a_dir = a.path().is_dir();
        let b_dir = b.path().is_dir();
        b_dir.cmp(&a_dir).then(a.file_name().cmp(&b.file_name()))
    });

    let mut nodes = Vec::new();
    for entry in entries.iter().take(MAX_ENTRIES_PER_DIR) {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();

        // Validate that this path is still within project root (symlink guard).
        if let Ok(canonical) = crate::path_utils::canonicalize_normalized(&path) {
            if !canonical.starts_with(root) {
                continue; // symlink escaping project root — skip
            }
        }

        let (kind, children) = if path.is_dir() {
            let children = scan_directory(root, &path, depth + 1).unwrap_or_default();
            (FileKind::Directory, children)
        } else {
            (FileKind::File, vec![])
        };

        nodes.push(FileNode { name, path, kind, children });
    }

    Ok(nodes)
}

/// Load simple gitignore patterns from a directory's `.gitignore`.
/// Only supports plain filename/glob patterns (no negation, no `**`).
fn load_gitignore_patterns(dir: &Path) -> Vec<String> {
    let gi_path = dir.join(".gitignore");
    if !gi_path.exists() {
        return vec![];
    }
    std::fs::read_to_string(gi_path)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|l| l.trim().trim_end_matches('/').to_string())
        .collect()
}

fn is_ignored(name: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|p| name == p.as_str())
}
