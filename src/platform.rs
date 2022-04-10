use std::fmt;
use std::fmt::Formatter;
use std::str::FromStr;

/// Various platforms supported in SVM.
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum Platform {
    /// target `x86_64-unknown-linux-gnu`.
    LinuxAmd64,
    /// target `aarch64-unknown-linux-gnu`.
    LinuxAarch64,
    /// target `x86_64-apple-darwin`.
    MacOsAmd64,
    /// target `aarch64-apple-darwin`.
    MacOsAarch64,
    /// target `x86_64-pc-windows-gnu`.
    WindowsAmd64,
    /// Variant to denote that current machine is not supported.
    Unsupported,
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Platform::LinuxAmd64 => "linux-amd64",
            Platform::LinuxAarch64 => "linux-aarch64",
            Platform::MacOsAmd64 => "macosx-amd64",
            Platform::MacOsAarch64 => "macosx-aarch64",
            Platform::WindowsAmd64 => "windows-amd64",
            Platform::Unsupported => "Unsupported-platform",
        };
        f.write_str(s)
    }
}

impl FromStr for Platform {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "linux-amd64" => Ok(Platform::LinuxAmd64),
            "linux-aarch64" => Ok(Platform::LinuxAarch64),
            "macosx-amd64" => Ok(Platform::MacOsAmd64),
            "macosx-aarch64" => Ok(Platform::MacOsAarch64),
            "windows-amd64" => Ok(Platform::WindowsAmd64),
            s => Err(format!("unsupported platform {}", s)),
        }
    }
}

/// Read the current machine's platform.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub fn platform() -> Platform {
    Platform::LinuxAmd64
}

/// Read the current machine's platform.
#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
pub fn platform() -> Platform {
    Platform::LinuxAarch64
}

/// Read the current machine's platform.
#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
pub fn platform() -> Platform {
    Platform::MacOsAmd64
}

/// Read the current machine's platform.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub fn platform() -> Platform {
    Platform::MacOsAarch64
}

/// Read the current machine's platform.
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub fn platform() -> Platform {
    Platform::WindowsAmd64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    fn get_platform() {
        assert_eq!(platform(), Platform::LinuxAmd64);
    }

    #[test]
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    fn get_platform() {
        assert_eq!(platform(), Platform::LinuxAarch64);
    }

    #[test]
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    fn get_platform() {
        assert_eq!(platform(), Platform::MacOsAmd64);
    }

    #[test]
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    fn get_platform() {
        assert_eq!(platform(), Platform::MacOsAarch64);
    }

    #[test]
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    fn get_platform() {
        assert_eq!(platform(), Platform::WindowsAmd64);
    }
}
