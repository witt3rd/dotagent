use crate::repo::{discover_repos, Config, RepoInfo};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Dashboard,
    Log,
    LogDetail,
    State,
    KillConfirm,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub filename: String,
    pub event_type: char,
    pub subject: String,
    pub body: String,
}

pub struct App {
    pub repos: Vec<RepoInfo>,
    pub selected: usize,
    pub mode: Mode,
    pub log_selected: usize,
    pub log_entries: Vec<LogEntry>,
    pub detail_scroll: usize,
    pub config: Config,
    pub last_scan: std::time::Instant,
    pub bus_path: Option<String>,
    pub last_activity: Option<String>,
}

impl App {
    pub fn new(config: Config) -> Self {
        let repos = discover_repos(&config);
        App {
            repos,
            selected: 0,
            mode: Mode::Dashboard,
            log_selected: 0,
            log_entries: Vec::new(),
            detail_scroll: 0,
            config,
            last_scan: std::time::Instant::now(),
            bus_path: None,
            last_activity: None,
        }
    }

    pub fn refresh(&mut self) {
        self.repos = discover_repos(&self.config);
        self.last_scan = std::time::Instant::now();
        if self.selected >= self.repos.len() && !self.repos.is_empty() {
            self.selected = self.repos.len() - 1;
        }
    }

    pub fn load_log(&mut self) {
        self.log_entries.clear();
        self.log_selected = 0;
        self.detail_scroll = 0;
        if let Some(repo) = self.selected_repo() {
            let log_dir = repo.path.join(".agent/log");
            if !log_dir.exists() {
                return;
            }
            let mut files: Vec<String> = std::fs::read_dir(&log_dir)
                .ok()
                .map(|rd| {
                    rd.filter_map(|e| e.ok())
                        .map(|e| e.file_name().to_string_lossy().to_string())
                        .filter(|f| f.ends_with(".md"))
                        .collect()
                })
                .unwrap_or_default();
            files.sort_by(|a, b| b.cmp(a)); // most recent first
            for f in files {
                let content = std::fs::read_to_string(log_dir.join(&f)).unwrap_or_default();
                let event_type = f.chars().next().unwrap_or('?');
                let subject = content
                    .lines()
                    .find(|l| l.starts_with("subject:"))
                    .map(|l| l.trim_start_matches("subject:").trim().to_string())
                    .unwrap_or_default();
                self.log_entries.push(LogEntry {
                    filename: f,
                    event_type,
                    subject,
                    body: content,
                });
            }
        }
    }

    pub fn selected_repo(&self) -> Option<&RepoInfo> {
        self.repos.get(self.selected)
    }

    pub fn next(&mut self) {
        if !self.repos.is_empty() {
            self.selected = (self.selected + 1) % self.repos.len();
            self.log_selected = 0;
        }
    }

    pub fn prev(&mut self) {
        if !self.repos.is_empty() {
            self.selected = if self.selected == 0 {
                self.repos.len() - 1
            } else {
                self.selected - 1
            };
            self.log_selected = 0;
        }
    }

    pub fn log_up(&mut self) {
        if self.log_selected > 0 {
            self.log_selected -= 1;
        }
    }

    pub fn log_down(&mut self) {
        if !self.log_entries.is_empty() && self.log_selected + 1 < self.log_entries.len() {
            self.log_selected += 1;
        }
    }

    pub fn detail_up(&mut self) {
        self.detail_scroll = self.detail_scroll.saturating_sub(1);
    }

    pub fn detail_down(&mut self) {
        self.detail_scroll = self.detail_scroll.saturating_add(1);
    }
}
