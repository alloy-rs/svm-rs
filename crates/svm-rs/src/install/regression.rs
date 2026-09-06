//! These tests use nextest's process isolation; workers within each test deliberately share a cache.
use super::*;
use std::os::unix::fs::MetadataExt;
#[cfg(target_os = "linux")]
use std::{
    process::{Child, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Instant,
};

fn executable() -> Vec<u8> {
    let output = Command::new("which").arg("true").output().unwrap();
    assert!(output.status.success());
    fs::read(String::from_utf8(output.stdout).unwrap().trim()).unwrap()
}

#[test]
fn nixos_cache_reuse_respects_patch_version_boundaries() {
    let bytes = executable();
    let checksum = sha2::Sha256::digest(&bytes);
    for (version, needs_patch) in [
        (Version::new(0, 7, 5), false),
        (Version::new(0, 7, 6), true),
        (Version::new(0, 8, 18), true),
        (Version::new(0, 8, 28), true),
        (Version::new(0, 8, 29), false),
    ] {
        // Model a checksum-valid, executable artifact restored from a non-NixOS cache. Do not use
        // do_install: on a NixOS host that would already patch the fixture before this test.
        setup_version(&version.to_string()).unwrap();
        let path = version_binary(&version.to_string());
        fs::write(&path, &bytes).unwrap();
        fs::set_permissions(&path, Permissions::from_mode(0o755)).unwrap();
        assert_eq!(requires_nixos_patch(&version, true), needs_patch);
        assert!(!requires_nixos_patch(&version, false));
        let _lock = try_lock_file(&lock_file_path(&version)).unwrap();
        for repair_permissions in [false, true] {
            let nixos = find_reusable_installation_for_platform(
                &version,
                &checksum,
                repair_permissions,
                true,
            )
            .unwrap();
            assert_eq!(nixos.is_none(), needs_patch, "NixOS solc {version}");
            let linux = find_reusable_installation_for_platform(
                &version,
                &checksum,
                repair_permissions,
                false,
            )
            .unwrap();
            assert_eq!(linux, Some(path.clone()), "non-NixOS solc {version}");
        }
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
}

#[test]
fn repairs_permissions_while_binary_is_running() {
    let version = Version::new(99, 0, 5);
    let output = Command::new("which").arg("sleep").output().unwrap();
    assert!(output.status.success());
    let bytes = fs::read(String::from_utf8(output.stdout).unwrap().trim()).unwrap();
    let checksum = sha2::Sha256::digest(&bytes);
    let path = do_install(&version, &bytes, "").unwrap();
    let mut child = Command::new(&path).arg("30").spawn().unwrap();
    // Reap the child before asserting the repair result, including on an error.
    let result = (|| {
        fs::set_permissions(&path, Permissions::from_mode(0o644))?;
        let _lock = try_lock_file(&lock_file_path(&version))?;
        do_install_and_retry(&version, &bytes, "", &checksum)
    })();
    let _ = child.kill();
    child.wait().unwrap();
    assert_eq!(result.unwrap(), path);
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o755
    );
}

#[test]
fn repairs_permissions_without_replacing_binary() {
    let version = Version::new(99, 0, 1);
    let bytes = executable();
    let checksum = sha2::Sha256::digest(&bytes);
    let path = do_install(&version, &bytes, "").unwrap();
    let inode = fs::metadata(&path).unwrap().ino();
    for mode in [0o644, 0o744, 0o654] {
        fs::set_permissions(&path, Permissions::from_mode(mode)).unwrap();
        assert!(
            find_reusable_installation(&version, &checksum, false)
                .unwrap()
                .is_none()
        );
        // The unlocked fast path must not mutate permissions.
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            mode
        );
        let _lock = try_lock_file(&lock_file_path(&version)).unwrap();
        assert_eq!(
            do_install_and_retry(&version, &bytes, "", &checksum).unwrap(),
            path
        );
        let metadata = fs::metadata(&path).unwrap();
        assert_eq!(metadata.ino(), inode);
        assert_eq!(metadata.permissions().mode() & 0o777, 0o755);
        assert!(Command::new(&path).status().unwrap().success());
    }
}

#[test]
fn permission_repair_does_not_trust_corrupt_contents() {
    let version = Version::new(99, 0, 2);
    let bytes = executable();
    let checksum = sha2::Sha256::digest(&bytes);
    let path = do_install(&version, b"corrupt", "").unwrap();
    fs::set_permissions(&path, Permissions::from_mode(0o644)).unwrap();
    let _lock = try_lock_file(&lock_file_path(&version)).unwrap();
    assert!(
        find_reusable_installation(&version, &checksum, true)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o644
    );
    do_install_and_retry(&version, &bytes, "", &checksum).unwrap();
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert!(Command::new(&path).status().unwrap().success());
}

#[test]
fn queued_lock_waiter_excludes_new_arrivals() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".lock");
    let first = try_lock_file(&path).unwrap();
    // A queued waiter has already opened this inode when the original holder releases it.
    let waiter = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    assert!(matches!(
        waiter.try_lock(),
        Err(fs::TryLockError::WouldBlock)
    ));
    drop(first);
    waiter.lock().unwrap();
    let newcomer = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    assert!(
        matches!(newcomer.try_lock(), Err(fs::TryLockError::WouldBlock)),
        "removing the lock file allowed two holders of the same logical lock"
    );
}

#[test]
fn orphaned_lock_is_not_current() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".lock");
    let old = try_lock_file(&path).unwrap();
    fs::remove_file(&path).unwrap();
    assert!(!lock_is_current(&old, &path).unwrap());
    let current = try_lock_file(&path).unwrap();
    assert!(!lock_is_current(&old, &path).unwrap());
    assert!(lock_is_current(&current, &path).unwrap());
}

#[test]
fn prepared_binary_has_no_writable_handle() {
    let version = Version::new(99, 0, 3);
    setup_version(&version.to_string()).unwrap();
    let bytes = executable();
    let prepared = Installer {
        version: &version,
        binbytes: &bytes,
    }
    .prepare()
    .unwrap();
    assert!(!version_binary(&version.to_string()).exists());
    assert!(Command::new(&prepared).status().unwrap().success());
}

// Ensure worker processes are cleaned up even if an assertion fails.
#[cfg(target_os = "linux")]
struct Worker(Child);

#[cfg(target_os = "linux")]
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

// Linux denies exec while a writable descriptor exists. Start readers before any writes so they
// cannot inherit a writer's descriptor: this specifically tests the installer's publication window.
#[cfg(target_os = "linux")]
#[test]
fn publication_with_independent_readers() {
    const WORKER_PATH: &str = "SVM_PUBLICATION_TEST_PATH";
    const READERS: usize = 4;
    const EXECUTIONS: usize = 500;
    if let Some(path) = std::env::var_os(WORKER_PATH) {
        fs::write(
            std::env::var_os("SVM_PUBLICATION_TEST_READY").unwrap(),
            b"ready",
        )
        .unwrap();
        std::io::stdin().read_exact(&mut [0]).unwrap();
        for _ in 0..EXECUTIONS {
            assert!(Command::new(&path).status().unwrap().success());
        }
        return;
    }

    let version = Version::new(99, 0, 4);
    let bytes = executable();
    let path = do_install(&version, &bytes, "").unwrap();
    let ready_dir = tempfile::tempdir().unwrap();
    let mut workers = Vec::new();
    for i in 0..READERS {
        let ready = ready_dir.path().join(i.to_string());
        workers.push(Worker(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "install::regression::publication_with_independent_readers",
                    "--nocapture",
                ])
                .env(WORKER_PATH, &path)
                .env("SVM_PUBLICATION_TEST_READY", &ready)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap(),
        ));
        let deadline = Instant::now() + Duration::from_secs(30);
        while !ready.exists() {
            assert!(Instant::now() < deadline, "reader failed to start");
            thread::sleep(Duration::from_millis(1));
        }
    }

    let done = AtomicBool::new(false);
    thread::scope(|scope| {
        let writer = scope.spawn(|| {
            let mut installs = 0;
            while !done.load(Ordering::Relaxed) {
                do_install(&version, &bytes, "").unwrap();
                installs += 1;
            }
            installs
        });
        for worker in &mut workers {
            if let Err(error) = worker.0.stdin.take().unwrap().write_all(b"x") {
                done.store(true, Ordering::Relaxed);
                panic!("failed to release reader: {error}");
            }
        }
        // Always stop the writer before asserting results, so a failed reader cannot hang the test.
        let results = workers
            .iter_mut()
            .map(|worker| worker.0.wait())
            .collect::<Vec<_>>();
        done.store(true, Ordering::Relaxed);
        let installs = writer.join().unwrap();
        assert!(installs > 0);
        for result in results {
            assert!(result.unwrap().success());
        }
        eprintln!(
            "{installs} replacements; {} successful executions",
            READERS * EXECUTIONS
        );
    });
}
