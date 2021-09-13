use std::env;

/// Types of supported platforms.
#[derive(Clone, Debug, PartialEq)]
pub enum Platform {
    LinuxAmd64,
    MacOsAmd64,
    Unsupported,
}

impl ToString for Platform {
    fn to_string(&self) -> String {
        match self {
            Platform::LinuxAmd64 => "linux-amd64".to_string(),
            Platform::MacOsAmd64 => "macosx-amd64".to_string(),
            Platform::Unsupported => "Unsupported-platform".to_string(),
        }
    }
}

/// Read the current machine's platform.
pub fn platform() -> Platform {
    match env::consts::OS {
        "linux" => Platform::LinuxAmd64,
        "macos" => Platform::MacOsAmd64,
        _ => Platform::Unsupported,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "linux")]
    fn get_platform() {
        assert_eq!(platform(), Platform::LinuxAmd64);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn get_platform() {
        assert_eq!(platform(), Platform::MacOsAmd64);
    }
}
