use std::env;

/// Types of supported platforms.
#[derive(Clone, Debug, PartialEq)]
pub enum Platform {
    LinuxAmd64,
    LinuxAarch64,
    MacOsAmd64,
    MacOsAarch64,
    WindowsAmd64,
    Unsupported,
}

impl ToString for Platform {
    fn to_string(&self) -> String {
        match self {
            Platform::LinuxAmd64 => "linux-amd64".to_string(),
            Platform::LinuxAarch64 => "linux-aarch64".to_string(),
            Platform::MacOsAmd64 => "macosx-amd64".to_string(),
            Platform::MacOsAarch64 => "macosx-aarch64".to_string(),
            Platform::WindowsAmd64 => "windows-amd64".to_string(),
            Platform::Unsupported => "Unsupported-platform".to_string(),
        }
    }
}

/// Read the current machine's platform.
pub fn platform() -> Platform {
    match (env::consts::OS, env::consts::ARCH) {
        ("linux", "x86_64") => Platform::LinuxAmd64,
        ("linux", "aarch64") => Platform::LinuxAarch64,
        ("macos", "x86_64") => Platform::MacOsAmd64,
        ("macos", "aarch64") => Platform::MacOsAarch64,
        ("windows", "x86_64") => Platform::WindowsAmd64,
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
}
