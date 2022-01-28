use std::env;

/// Types of supported platforms.
#[derive(Clone, Debug, PartialEq)]
pub enum Platform {
    LinuxAmd64,
    MacOsAmd64,
    WindowsAmd64,
    Unsupported,
}

impl ToString for Platform {
    fn to_string(&self) -> String {
        match self {
            Platform::LinuxAmd64 => "linux-amd64".to_string(),
            Platform::MacOsAmd64 => "macosx-amd64".to_string(),
            Platform::WindowsAmd64 => "windows-amd64".to_string(),
            Platform::Unsupported => "Unsupported-platform".to_string(),
        }
    }
}

/// Read the current machine's platform.
pub fn platform() -> Platform {
    match (env::consts::OS, env::consts::ARCH) {
        ("linux", "x86_64") => Platform::LinuxAmd64,
        // NOTE: Relaxed requirement on target architecture here
        // to support M1 macs with Rosetta
        ("macos", _) => Platform::MacOsAmd64,
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
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    fn get_platform() {
        assert_eq!(platform(), Platform::MacOsAmd64);
    }

    #[test]
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    fn get_platform() {
        assert_eq!(platform(), Platform::MacOsAmd64);
    }

    #[test]
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    fn get_platform() {
        assert_eq!(platform(), Platform::WindowsAmd64);
    }
}
