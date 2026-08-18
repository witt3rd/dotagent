use crate::repo::{LockState, RepoInfo};
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::fs;
use std::thread;
use std::time::Duration;

pub fn kill_agent(repo: &RepoInfo) -> Result<String, String> {
    let pid = match &repo.lock {
        LockState::Held { pid, .. } => *pid,
        _ => return Err("No active agent to kill".to_string()),
    };
    let nix_pid = Pid::from_raw(pid as i32);

    // try SIGTERM first (graceful)
    let _ = signal::kill(nix_pid, Signal::SIGTERM);
    thread::sleep(Duration::from_secs(3));

    // check if still alive
    let alive = unsafe { libc::kill(pid as i32, 0) } == 0;
    if alive {
        let _ = signal::kill(nix_pid, Signal::SIGKILL);
        thread::sleep(Duration::from_secs(1));
    }

    // remove the lock
    let lock_dir = repo.path.join(".agent/.busy");
    if lock_dir.exists() {
        let _ = fs::remove_dir_all(&lock_dir);
    }

    Ok(format!("Killed agent PID {} in {}", pid, repo.identity))
}
