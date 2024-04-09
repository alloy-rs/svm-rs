use crate::SvmError;
use std::{
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    sync::OnceLock,
};

/// Setup SVM home directory.
pub fn setup_data_dir() -> Result<(), SvmError> {
    // create $XDG_DATA_HOME or ~/.local/share/svm, or fallback to ~/.svm
    let data_dir = data_dir();

    // Create the directory, continuing if the directory came into existence after the check
    // for this if statement. This may happen if two copies of SVM run simultaneously (e.g CI).
    fs::create_dir_all(data_dir).or_else(|err| match err.kind() {
        io::ErrorKind::AlreadyExists => Ok(()),
        _ => Err(err),
    })?;

    // Check that the SVM directory is indeed a directory, and not e.g. a file.
    if !data_dir.is_dir() {
        return Err(SvmError::IoError(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("svm data dir '{}' is not a directory", data_dir.display()),
        )));
    }

    // Create `$SVM/.global-version`.
    let global_version = global_version_path();
    if !global_version.exists() {
        fs::File::create(global_version)?;
    }

    Ok(())
}

/// Returns the path to the default data directory.
///
/// Returns `~/.svm` if it exists, otherwise uses `$XDG_DATA_HOME/svm`.
pub fn data_dir() -> &'static Path {
    static ONCE: OnceLock<PathBuf> = OnceLock::new();
    ONCE.get_or_init(|| {
        #[cfg(test)]
        {
            let dir = tempfile::tempdir().expect("could not create temp directory");
            dir.path().join(".svm")
        }
        #[cfg(not(test))]
        {
            resolve_data_dir()
        }
    })
}

fn resolve_data_dir() -> PathBuf {
    let home_dir = dirs::home_dir()
        .expect("could not detect user home directory")
        .join(".svm");

    let data_dir = dirs::data_dir().expect("could not detect user data directory");
    if !home_dir.exists() && data_dir.exists() {
        data_dir.join("svm")
    } else {
        home_dir
    }
}

/// Returns the path to the global version file.
pub fn global_version_path() -> &'static Path {
    static ONCE: OnceLock<PathBuf> = OnceLock::new();
    ONCE.get_or_init(|| data_dir().join(".global-version"))
}

/// Returns the path to a specific Solc version's directory.
///
/// Note that this is not the path to the actual Solc binary file;
/// use [`version_binary`] for that instead.
///
/// This is currently `data_dir() / {version}`.
pub fn version_path(version: &str) -> PathBuf {
    data_dir().join(version)
}

/// Derive path to a specific Solc version's binary file.
///
/// This is currently `data_dir() / {version} / solc-{version}`.
pub fn version_binary(version: &str) -> PathBuf {
    let data_dir = data_dir();
    let sep = std::path::MAIN_SEPARATOR_STR;
    let cap =
        data_dir.as_os_str().len() + sep.len() + version.len() + sep.len() + 5 + version.len();
    let mut binary = OsString::with_capacity(cap);
    binary.push(data_dir);
    debug_assert!(!data_dir.ends_with(sep));
    binary.push(sep);

    binary.push(version);
    binary.push(sep);

    binary.push("solc-");
    binary.push(version);
    PathBuf::from(binary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_dir_resolution() {
        let home_dir = dirs::home_dir().unwrap().join(".svm");
        let data_dir = dirs::data_dir();
        let resolved_dir = resolve_data_dir();
        if home_dir.exists() || data_dir.is_none() {
            assert_eq!(resolved_dir, home_dir);
        } else {
            assert_eq!(resolved_dir, data_dir.unwrap().join("svm"));
        }
    }
}
