use std::fmt::Formatter;
use std::str::FromStr;
use std::{env, fmt};

/// Types of supported platforms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    LinuxAmd64,
    LinuxAarch64,
    MacOsAmd64,
    MacOsAarch64,
    WindowsAmd64,
    AndroidAarch64,
    Unsupported,
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::LinuxAmd64 => "linux-amd64",
            Self::LinuxAarch64 => "linux-aarch64",
            Self::MacOsAmd64 => "macosx-amd64",
            Self::MacOsAarch64 => "macosx-aarch64",
            Self::WindowsAmd64 => "windows-amd64",
            Self::AndroidAarch64 => "android-aarch64",
            Self::Unsupported => "Unsupported-platform",
        };
        f.write_str(s)
    }
}

impl FromStr for Platform {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "linux-amd64" => Ok(Self::LinuxAmd64),
            "linux-aarch64" => Ok(Self::LinuxAarch64),
            "macosx-amd64" => Ok(Self::MacOsAmd64),
            "macosx-aarch64" => Ok(Self::MacOsAarch64),
            "windows-amd64" => Ok(Self::WindowsAmd64),
            "android-aarch64" => Ok(Self::AndroidAarch64),
            s => Err(format!("unsupported platform {s}")),
        }
    }
}

pub fn is_nixos() -> bool {
    cfg!(target_os = "linux")
        && (std::path::Path::new("/etc/nixos").exists()
            || std::path::Path::new("/etc/NIXOS").exists())
        && std::fs::read_to_string("/etc/os-release").is_ok_and(|s| s.contains("NixOS"))
}

/// Read the current machine's platform.
pub fn platform() -> Platform {
    match (env::consts::OS, env::consts::ARCH) {
        ("linux", "x86_64") => Platform::LinuxAmd64,
        ("linux", "aarch64") => Platform::LinuxAarch64,
        ("macos", "x86_64") => Platform::MacOsAmd64,
        ("macos", "aarch64") => Platform::MacOsAarch64,
        ("windows", "x86_64") => Platform::WindowsAmd64,
        ("android", "aarch64") => Platform::AndroidAarch64,
        _ => Platform::Unsupported,
    }
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

    #[test]
    #[cfg(all(target_os = "android", target_arch = "aarch64"))]
    fn get_platform() {
        assert_eq!(platform(), Platform::AndroidAarch64);
    }
}
