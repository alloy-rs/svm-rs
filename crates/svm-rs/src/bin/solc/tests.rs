use super::*;

#[test]
fn retries_only_busy_launches() {
    let mut attempts = 0;
    let result = retry_busy(|| {
        attempts += 1;
        if attempts == 1 {
            Err(io::Error::from(io::ErrorKind::ExecutableFileBusy))
        } else {
            Ok(42)
        }
    })
    .unwrap();
    assert_eq!(result, 42);
    assert_eq!(attempts, 2);

    for kind in [io::ErrorKind::NotFound, io::ErrorKind::PermissionDenied] {
        let mut attempts = 0;
        let error = retry_busy::<()>(|| {
            attempts += 1;
            Err(io::Error::from(kind))
        })
        .unwrap_err();
        assert_eq!(error.kind(), kind);
        assert_eq!(attempts, 1);
    }
}

#[test]
fn busy_retries_are_bounded() {
    let mut attempts = 0;
    let error = retry_busy::<()>(|| {
        attempts += 1;
        Err(io::Error::from(io::ErrorKind::ExecutableFileBusy))
    })
    .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::ExecutableFileBusy);
    assert_eq!(attempts, 6);
}

#[cfg(unix)]
#[test]
fn compiler_failure_is_not_retried() {
    let mut attempts = 0;
    let status = retry_busy(|| {
        attempts += 1;
        Command::new("sh").args(["-c", "exit 42"]).spawn()
    })
    .unwrap()
    .wait()
    .unwrap();
    assert_eq!(status.code(), Some(42));
    assert_eq!(attempts, 1);
}

#[cfg(target_os = "linux")]
#[test]
fn recovers_after_inherited_write_handle_is_closed() {
    use std::{fs, os::unix::fs::PermissionsExt};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("solc");
    fs::copy("/bin/true", &path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    let original = fs::OpenOptions::new().write(true).open(&path).unwrap();
    let mut duplicate = Some(original.try_clone().unwrap());
    drop(original);
    let mut attempts = 0;
    let child = retry_busy(|| {
        attempts += 1;
        let result = Command::new(&path).spawn();
        if attempts == 1 {
            assert_eq!(
                result.as_ref().unwrap_err().kind(),
                io::ErrorKind::ExecutableFileBusy
            );
            // Model the forked child closing its duplicate during exec, after our rejected launch.
            drop(duplicate.take());
        }
        result
    })
    .unwrap();
    assert!(child.wait_with_output().unwrap().status.success());
    assert_eq!(attempts, 2);
}

#[cfg(unix)]
#[test]
fn exec_preserves_arguments_output_and_exit_status() {
    const WORKER: &str = "SVM_EXEC_TEST_WORKER";
    if std::env::var_os(WORKER).is_some() {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "printf '%s' \"$1\"; exit 42", "sh", "compiler output"]);
        // A successful Unix exec replaces this test process and never returns.
        panic!("exec returned: {:?}", exec(&mut cmd));
    }
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "tests::exec_preserves_arguments_output_and_exit_status",
            "--nocapture",
        ])
        .env(WORKER, "1")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(42));
    assert!(String::from_utf8_lossy(&output.stdout).contains("compiler output"));
}
