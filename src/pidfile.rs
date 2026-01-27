//! PID file management for daemon lifecycle
//!
//! This module provides utilities for creating, validating, and cleaning up PID files.
//! PID files are used to:
//! - Prevent multiple instances of the daemon from running
//! - Allow other processes to identify the daemon
//! - Clean up resources on shutdown
//!
//! ## Usage
//!
//! ```ignore
//! let pidfile = PidFile::create("/var/run/surface-dial.pid")?;
//! // Daemon runs...
//! drop(pidfile); // PID file removed on drop
//! ```

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during PID file operations
#[derive(Debug, Error)]
pub enum PidFileError {
    /// Another instance of the daemon is already running
    #[error("Another instance is already running (PID: {0})")]
    AlreadyRunning(u32),

    /// Failed to create or write the PID file
    #[error("Failed to create PID file: {0}")]
    CreateFailed(#[from] io::Error),

    /// PID file exists but contains invalid data
    #[error("Invalid PID file contents: {0}")]
    InvalidContents(String),

    /// Failed to create parent directory
    #[error("Failed to create PID file directory: {0}")]
    DirectoryError(io::Error),
}

/// Result type for PID file operations
pub type PidFileResult<T> = Result<T, PidFileError>;

/// Represents an active PID file
///
/// The PID file is automatically removed when this struct is dropped,
/// ensuring cleanup even on panic.
pub struct PidFile {
    /// Path to the PID file
    path: PathBuf,
    /// Whether to remove the file on drop
    remove_on_drop: bool,
}

impl PidFile {
    /// Create a new PID file at the given path
    ///
    /// This will:
    /// 1. Check if a stale PID file exists and clean it up
    /// 2. Create the parent directory if needed
    /// 3. Write the current process ID to the file
    /// 4. Set appropriate file permissions (readable by all, writable by owner)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Another instance is already running (the PID in the file is alive)
    /// - Failed to create the file or directory
    pub fn create<P: AsRef<Path>>(path: P) -> PidFileResult<Self> {
        let path = path.as_ref().to_path_buf();

        // Check for existing PID file
        if path.exists() {
            match Self::check_stale(&path) {
                Ok(None) => {
                    // Stale file, remove it
                    let _ = fs::remove_file(&path);
                }
                Ok(Some(pid)) => {
                    return Err(PidFileError::AlreadyRunning(pid));
                }
                Err(_) => {
                    // Invalid file, remove it
                    let _ = fs::remove_file(&path);
                }
            }
        }

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(PidFileError::DirectoryError)?;
            }
        }

        // Create and write PID file
        let pid = std::process::id();
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;

        writeln!(file, "{}", pid)?;
        file.sync_all()?;

        // Set file permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o644); // rw-r--r--
            fs::set_permissions(&path, permissions)?;
        }

        Ok(Self {
            path,
            remove_on_drop: true,
        })
    }

    /// Create a PID file that won't be removed on drop
    ///
    /// Useful for testing or when the file should persist.
    pub fn create_persistent<P: AsRef<Path>>(path: P) -> PidFileResult<Self> {
        let mut pidfile = Self::create(path)?;
        pidfile.remove_on_drop = false;
        Ok(pidfile)
    }

    /// Check if a PID file is stale (the process is no longer running)
    ///
    /// Returns:
    /// - `Ok(None)` if the file is stale
    /// - `Ok(Some(pid))` if the process is still running
    /// - `Err` if the file is invalid or unreadable
    pub fn check_stale<P: AsRef<Path>>(path: P) -> PidFileResult<Option<u32>> {
        let contents = fs::read_to_string(path.as_ref())?;
        let pid: u32 = contents
            .trim()
            .parse()
            .map_err(|_| PidFileError::InvalidContents(contents.clone()))?;

        if Self::is_process_alive(pid) {
            Ok(Some(pid))
        } else {
            Ok(None)
        }
    }

    /// Read the PID from an existing PID file
    pub fn read_pid<P: AsRef<Path>>(path: P) -> PidFileResult<u32> {
        let contents = fs::read_to_string(path.as_ref())?;
        contents
            .trim()
            .parse()
            .map_err(|_| PidFileError::InvalidContents(contents))
    }

    /// Get the path to this PID file
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the PID stored in this file
    pub fn pid(&self) -> PidFileResult<u32> {
        Self::read_pid(&self.path)
    }

    /// Manually remove the PID file
    pub fn remove(&mut self) -> io::Result<()> {
        if self.path.exists() {
            fs::remove_file(&self.path)?;
        }
        self.remove_on_drop = false;
        Ok(())
    }

    /// Check if a process with the given PID is alive
    #[cfg(unix)]
    fn is_process_alive(pid: u32) -> bool {
        // On Unix, use kill -0 via shell to check if process exists
        // This is safer than using libc directly
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[cfg(not(unix))]
    fn is_process_alive(pid: u32) -> bool {
        // On Windows, we'd use OpenProcess
        // For now, assume process is alive if we can't check
        // This is conservative - we won't overwrite a potentially running instance
        true
    }

    /// Verify the PID file contains the expected PID
    pub fn verify(&self, expected_pid: u32) -> bool {
        self.pid().map(|p| p == expected_pid).unwrap_or(false)
    }
}

impl Drop for PidFile {
    fn drop(&mut self) {
        if self.remove_on_drop {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// Default path for the daemon PID file
pub fn default_pid_path() -> PathBuf {
    dirs::runtime_dir()
        .or_else(|| dirs::data_local_dir())
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("surface-dial")
        .join("daemon.pid")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ==========================================================================
    // PID File Creation Tests
    // ==========================================================================

    #[test]
    fn test_pid_file_creation() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("test.pid");

        let pidfile = PidFile::create(&path).unwrap();

        // File should exist
        assert!(path.exists(), "PID file should be created");

        // File should contain current PID
        let contents = fs::read_to_string(&path).unwrap();
        let stored_pid: u32 = contents.trim().parse().unwrap();
        assert_eq!(stored_pid, std::process::id());

        drop(pidfile);
    }

    #[test]
    fn test_pid_file_creates_parent_directories() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("nested").join("deep").join("test.pid");

        // Parent directories don't exist
        assert!(!temp.path().join("nested").exists());

        let pidfile = PidFile::create(&path).unwrap();
        assert!(path.exists());
        assert!(temp.path().join("nested").join("deep").exists());

        drop(pidfile);
    }

    #[test]
    fn test_pid_file_removed_on_drop() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("test.pid");

        {
            let _pidfile = PidFile::create(&path).unwrap();
            assert!(path.exists());
        }
        // After drop
        assert!(!path.exists(), "PID file should be removed on drop");
    }

    #[test]
    fn test_pid_file_persistent_not_removed() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("persistent.pid");

        {
            let _pidfile = PidFile::create_persistent(&path).unwrap();
            assert!(path.exists());
        }
        // After drop - should still exist
        assert!(path.exists(), "Persistent PID file should remain after drop");
    }

    // ==========================================================================
    // PID File Content Tests
    // ==========================================================================

    #[test]
    fn test_pid_file_contains_actual_pid() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("test.pid");

        let pidfile = PidFile::create(&path).unwrap();
        let stored = pidfile.pid().unwrap();

        assert_eq!(stored, std::process::id());

        drop(pidfile);
    }

    #[test]
    fn test_read_pid_from_file() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("test.pid");

        // Write a known PID
        fs::write(&path, "12345\n").unwrap();

        let pid = PidFile::read_pid(&path).unwrap();
        assert_eq!(pid, 12345);
    }

    #[test]
    fn test_read_pid_trims_whitespace() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("test.pid");

        fs::write(&path, "  54321  \n\n").unwrap();

        let pid = PidFile::read_pid(&path).unwrap();
        assert_eq!(pid, 54321);
    }

    #[test]
    fn test_invalid_pid_file_contents() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("invalid.pid");

        fs::write(&path, "not-a-number\n").unwrap();

        let result = PidFile::read_pid(&path);
        assert!(matches!(result, Err(PidFileError::InvalidContents(_))));
    }

    #[test]
    fn test_empty_pid_file() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("empty.pid");

        fs::write(&path, "").unwrap();

        let result = PidFile::read_pid(&path);
        assert!(matches!(result, Err(PidFileError::InvalidContents(_))));
    }

    // ==========================================================================
    // Stale PID Detection Tests
    // ==========================================================================

    #[test]
    fn test_check_stale_nonexistent_process() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("stale.pid");

        // Write a PID that almost certainly doesn't exist
        // Using a very high PID that's unlikely to be assigned
        fs::write(&path, "999999999\n").unwrap();

        let result = PidFile::check_stale(&path);
        // On Unix, this will return Ok(None) for stale
        // On other platforms, it may return Ok(Some) due to conservative check
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_stale_current_process() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("current.pid");

        // Write current process PID
        fs::write(&path, format!("{}\n", std::process::id())).unwrap();

        let result = PidFile::check_stale(&path).unwrap();
        assert_eq!(result, Some(std::process::id()));
    }

    #[test]
    fn test_check_stale_invalid_contents() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("invalid.pid");

        fs::write(&path, "garbage\n").unwrap();

        let result = PidFile::check_stale(&path);
        assert!(matches!(result, Err(PidFileError::InvalidContents(_))));
    }

    #[test]
    fn test_stale_pid_file_is_replaced() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("stale.pid");

        // Write a stale PID (very high, unlikely to exist)
        fs::write(&path, "999999999\n").unwrap();

        // Creating a new PID file should succeed and replace the stale one
        let pidfile = PidFile::create(&path).unwrap();
        let stored = pidfile.pid().unwrap();

        assert_eq!(stored, std::process::id());

        drop(pidfile);
    }

    // ==========================================================================
    // Race Condition and Cleanup Tests
    // ==========================================================================

    #[test]
    fn test_prevent_duplicate_creation() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("test.pid");

        // Write current process PID (simulating running daemon)
        fs::write(&path, format!("{}\n", std::process::id())).unwrap();

        // Trying to create should fail with AlreadyRunning
        let result = PidFile::create(&path);
        assert!(matches!(result, Err(PidFileError::AlreadyRunning(_))));
    }

    #[test]
    fn test_manual_remove() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("manual.pid");

        let mut pidfile = PidFile::create(&path).unwrap();
        assert!(path.exists());

        pidfile.remove().unwrap();
        assert!(!path.exists());

        // Double remove should be ok
        assert!(pidfile.remove().is_ok());
    }

    #[test]
    fn test_remove_prevents_drop_cleanup() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("nodelete.pid");

        {
            let mut pidfile = PidFile::create(&path).unwrap();
            pidfile.remove().unwrap();

            // Recreate the file manually
            fs::write(&path, "12345\n").unwrap();
        }
        // After drop - file should still exist (we called remove())
        assert!(path.exists());
    }

    // ==========================================================================
    // File Permissions Tests (Unix only)
    // ==========================================================================

    #[cfg(unix)]
    #[test]
    fn test_pid_file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp = TempDir::new().unwrap();
        let path = temp.path().join("perms.pid");

        let pidfile = PidFile::create(&path).unwrap();

        let metadata = fs::metadata(&path).unwrap();
        let mode = metadata.permissions().mode();

        // Should be 0644 (rw-r--r--)
        // Note: mode includes file type bits, so we mask with 0o777
        assert_eq!(mode & 0o777, 0o644);

        drop(pidfile);
    }

    // ==========================================================================
    // Path and Utility Tests
    // ==========================================================================

    #[test]
    fn test_path_accessor() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("accessor.pid");

        let pidfile = PidFile::create(&path).unwrap();
        assert_eq!(pidfile.path(), path.as_path());

        drop(pidfile);
    }

    #[test]
    fn test_verify_correct_pid() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("verify.pid");

        let pidfile = PidFile::create(&path).unwrap();
        assert!(pidfile.verify(std::process::id()));
        assert!(!pidfile.verify(12345)); // Wrong PID

        drop(pidfile);
    }

    #[test]
    fn test_default_pid_path_not_empty() {
        let path = default_pid_path();
        assert!(!path.as_os_str().is_empty());
        assert!(path.to_string_lossy().contains("surface-dial"));
        assert!(path.to_string_lossy().contains(".pid"));
    }

    // ==========================================================================
    // Error Handling Tests
    // ==========================================================================

    #[test]
    fn test_create_in_nonexistent_readonly_fails() {
        // This test tries to create in a path that should fail
        // /nonexistent is not writable by normal users
        let result = PidFile::create("/nonexistent/path/that/should/fail/test.pid");
        assert!(result.is_err());
    }

    #[test]
    fn test_read_nonexistent_file() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("nonexistent.pid");

        let result = PidFile::read_pid(&path);
        assert!(result.is_err());
    }

    // ==========================================================================
    // Integration-style Tests
    // ==========================================================================

    #[test]
    fn test_complete_lifecycle() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("lifecycle.pid");

        // 1. Create PID file
        let pidfile = PidFile::create(&path).unwrap();
        assert!(path.exists());

        // 2. Verify contents
        let pid = pidfile.pid().unwrap();
        assert_eq!(pid, std::process::id());

        // 3. Verify it's not stale
        let check = PidFile::check_stale(&path).unwrap();
        assert_eq!(check, Some(std::process::id()));

        // 4. Cannot create another
        let dup_result = PidFile::create(&path);
        assert!(matches!(dup_result, Err(PidFileError::AlreadyRunning(_))));

        // 5. Drop cleans up
        drop(pidfile);
        assert!(!path.exists());

        // 6. Can create again after cleanup
        let new_pidfile = PidFile::create(&path);
        assert!(new_pidfile.is_ok());
    }

    #[test]
    fn test_recovery_from_crash() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("crash.pid");

        // Simulate crashed daemon (stale PID file with non-running PID)
        fs::write(&path, "999999999\n").unwrap();
        assert!(path.exists());

        // New daemon should be able to start (replaces stale file)
        let pidfile = PidFile::create(&path).unwrap();
        let pid = pidfile.pid().unwrap();

        // Should now contain our PID, not the stale one
        assert_eq!(pid, std::process::id());
        assert_ne!(pid, 999999999);

        drop(pidfile);
    }
}
