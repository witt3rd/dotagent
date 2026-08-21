use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub repos: Vec<PathBuf>,
    #[serde(default)]
    pub scan_roots: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub enum Status {
    Active,
    Idle,
    Stale,
    Error,
}

#[derive(Debug, Clone)]
pub enum LockState {
    Free,
    Held { pid: u32, since: String },
    Stale { pid: u32, since: String },
}

#[derive(Debug, Clone)]
pub struct RepoInfo {
    pub path: PathBuf,
    pub identity: String,
    pub status: Status,
    pub lock: LockState,
    pub event_count: usize,
    pub open_inbound: usize,
    pub open_outbound: usize,
    pub handoffs: usize,
    pub is_worktree: bool,
    pub chain_depth: usize,
    pub last_event: Option<DateTime<Utc>>,
    pub last_handoff_subject: Option<String>,
}

impl RepoInfo {
    pub fn from_path(path: &Path) -> Option<Self> {
        // canonicalize to avoid duplicate entries from different path representations
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let agent_dir = path.join(".agent");
        let config_file = agent_dir.join("config");
        if !config_file.exists() {
            return None;
        }

        let identity = fs::read_to_string(&config_file)
            .ok()
            .and_then(|c| {
                c.lines()
                    .find(|l| l.starts_with("identity:"))
                    .map(|l| l.trim_start_matches("identity:").trim().to_string())
            })
            .unwrap_or_else(|| path.file_name().unwrap_or_default().to_string_lossy().to_string());

        let is_worktree = path.join(".git").is_file(); // worktrees have a .git file, not directory
        let log_dir = agent_dir.join("log");
        let events: Vec<String> = if log_dir.exists() {
            fs::read_dir(&log_dir)
                .ok()
                .map(|rd| {
                    rd.filter_map(|e| e.ok())
                        .map(|e| e.file_name().to_string_lossy().to_string())
                        .filter(|f| f.ends_with(".md"))
                        .collect()
                })
                .unwrap_or_default()
        } else {
            vec![]
        };

        let event_count = events.len();

        let open_inbound = events.iter().filter(|f| f.starts_with("I--")).count()
            - events
                .iter()
                .filter(|f| f.starts_with("C--") || f.starts_with("R--"))
                .count()
                .min(events.iter().filter(|f| f.starts_with("I--")).count());

        let open_outbound = events.iter().filter(|f| f.starts_with("O--")).count()
            - events
                .iter()
                .filter(|f| f.starts_with("R--"))
                .count()
                .min(events.iter().filter(|f| f.starts_with("O--")).count());

        let handoffs = events.iter().filter(|f| f.starts_with("H--")).count();
        let chain_depth = handoffs; // simplified: total handoffs = chain depth

        let last_event = events
            .iter()
            .filter_map(|f| {
                let ts_str = f
                    .split("--")
                    .nth(1)?
                    .split('-')
                    .next()? // strip the event ID (everything after the first '-')
                    .trim_end_matches(".md");
                DateTime::parse_from_str(&format!("{}Z", ts_str), "%Y-%m-%dT%H%M%SZ")
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            })
            .max();

        let last_handoff_subject = events
            .iter()
            .filter(|f| f.starts_with("H--"))
            .last()
            .and_then(|f| {
                let content = fs::read_to_string(log_dir.join(f)).ok()?;
                content
                    .lines()
                    .find(|l| l.starts_with("subject:"))
                    .map(|l| l.trim_start_matches("subject:").trim().to_string())
            });

        let lock = read_lock(&agent_dir);

        let status = match &lock {
            LockState::Held { .. } => Status::Active,
            LockState::Stale { .. } => Status::Error,
            LockState::Free => {
                if let Some(ts) = &last_event {
                    let age = Utc::now().signed_duration_since(*ts);
                    if age.num_days() > 7 {
                        Status::Stale
                    } else {
                        Status::Idle
                    }
                } else {
                    Status::Idle
                }
            }
        };

        Some(RepoInfo {
            path,
            identity,
            status,
            lock,
            event_count,
            open_inbound,
            open_outbound,
            handoffs,
            is_worktree,
            chain_depth,
            last_event,
            last_handoff_subject,
        })
    }
}

fn read_lock(agent_dir: &Path) -> LockState {
    let lock_dir = agent_dir.join(".busy");
    if !lock_dir.exists() {
        return LockState::Free;
    }
    let pid = fs::read_to_string(lock_dir.join("pid"))
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(0);
    let since = fs::read_to_string(lock_dir.join("ts"))
        .ok()
        .unwrap_or_default();

        if pid > 1 {
            // check if alive via libc::kill(pid, 0)
            let alive = unsafe { libc::kill(pid as i32, 0) } == 0;
            if alive {
                LockState::Held { pid, since }
            } else {
                LockState::Stale { pid, since }
            }
    } else {
        LockState::Free
    }
}

pub fn discover_repos(config: &Config) -> Vec<RepoInfo> {
    let mut repos: Vec<RepoInfo> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // explicit repos
    for path in &config.repos {
        let p = if path.is_relative() {
            std::env::current_dir().unwrap_or_default().join(path)
        } else {
            path.clone()
        };
        if let Some(info) = RepoInfo::from_path(&p) {
            if seen.insert(info.path.clone()) {
                repos.push(info);
            }
        }
    }

    // scan roots
    for root in &config.scan_roots {
        let root = if root.is_relative() {
            std::env::current_dir().unwrap_or_default().join(root)
        } else {
            root.clone()
        };
        if !root.exists() {
            continue;
        }
        for entry in WalkDir::new(&root)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_dir() && entry.path().join(".agent/config").exists() {
                if let Some(info) = RepoInfo::from_path(entry.path()) {
                    if seen.insert(info.path.clone()) {
                        repos.push(info);
                    }
                }
            }
        }
    }

    repos.sort_by(|a, b| a.identity.cmp(&b.identity));
    repos
}
