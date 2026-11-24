//! Executable lookup without pulling in a process spawning framework.

use std::env;
use std::path::{Path, PathBuf};

use crate::Result;

/// Find an executable in `PATH`.
pub fn find(name: &str) -> Option<PathBuf> {
    if name.contains('/') {
        let path = PathBuf::from(name);
        return is_executable(&path).then_some(path);
    }

    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| is_executable(candidate))
}

pub fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    path.metadata()
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// Resolve an executable or return an error that names the command.
pub fn require(name: &str) -> Result<PathBuf> {
    find(name).ok_or_else(|| crate::Error::CommandMissing(name.to_owned()))
}
