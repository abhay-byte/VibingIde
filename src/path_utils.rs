use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

pub fn canonicalize_normalized(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing {}", path.display()))?;
    Ok(normalize_platform_path(canonical))
}

pub fn normalize_platform_path(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let raw = path.to_string_lossy();
        if let Some(stripped) = raw.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{stripped}"));
        }
        if let Some(stripped) = raw.strip_prefix(r"\\?\") {
            return PathBuf::from(stripped);
        }
    }

    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn strips_windows_verbatim_prefix() {
        let path = PathBuf::from(r"\\?\C:\Users\abhay\repos\VibingIde");
        assert_eq!(
            normalize_platform_path(path),
            PathBuf::from(r"C:\Users\abhay\repos\VibingIde")
        );
    }

    #[cfg(windows)]
    #[test]
    fn strips_windows_unc_verbatim_prefix() {
        let path = PathBuf::from(r"\\?\UNC\server\share\repo");
        assert_eq!(
            normalize_platform_path(path),
            PathBuf::from(r"\\server\share\repo")
        );
    }
}
