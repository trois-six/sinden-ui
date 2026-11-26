use std::fmt;
use std::path::{Path, PathBuf};

/// Environment variable overriding the endpoint: a socket path on Linux and
/// macOS, a pipe name (`\\.\pipe\...`) on Windows.
pub const ENDPOINT_ENV: &str = "SINDEN_DRIVER_SOCKET";

/// Where the driver's control API listens.
///
/// | System | Endpoint |
/// |---|---|
/// | Linux (and other Unix) | `/run/sinden-driver/control.sock` |
/// | macOS | `~/Library/Application Support/SindenDriver/control.sock` |
/// | Windows | named pipe `\\.\pipe\sinden-driver` |
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    path: PathBuf,
}

impl Endpoint {
    /// An endpoint at an explicit path (socket path or pipe name).
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// The endpoint named by [`ENDPOINT_ENV`], or this system's default.
    #[must_use]
    pub fn from_env_or_default() -> Self {
        match std::env::var_os(ENDPOINT_ENV) {
            Some(value) if !value.is_empty() => Self::new(value),
            _ => Self::platform_default(),
        }
    }

    /// This system's default endpoint.
    #[must_use]
    pub fn platform_default() -> Self {
        Self::new(default_path(
            std::env::var_os("HOME").map(PathBuf::from).as_deref(),
        ))
    }

    /// The socket path or pipe name.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.path.display().fmt(f)
    }
}

#[cfg(target_os = "windows")]
fn default_path(_home: Option<&Path>) -> PathBuf {
    PathBuf::from(r"\\.\pipe\sinden-driver")
}

#[cfg(target_os = "macos")]
fn default_path(home: Option<&Path>) -> PathBuf {
    home.unwrap_or(Path::new("/"))
        .join("Library/Application Support/SindenDriver/control.sock")
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn default_path(_home: Option<&Path>) -> PathBuf {
    PathBuf::from("/run/sinden-driver/control.sock")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "linux")]
    fn linux_default() {
        assert_eq!(
            default_path(Some(Path::new("/home/someone"))),
            Path::new("/run/sinden-driver/control.sock")
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn macos_default_is_per_user() {
        assert_eq!(
            default_path(Some(Path::new("/Users/someone"))),
            Path::new("/Users/someone/Library/Application Support/SindenDriver/control.sock")
        );
    }

    #[test]
    #[cfg(windows)]
    fn windows_default() {
        assert_eq!(default_path(None), Path::new(r"\\.\pipe\sinden-driver"));
    }

    #[test]
    fn display_is_the_path() {
        assert_eq!(Endpoint::new("/x/y.sock").to_string(), "/x/y.sock");
    }
}
