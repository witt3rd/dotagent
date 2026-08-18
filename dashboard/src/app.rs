use crate::repo::{discover_repos, Config, RepoInfo};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Dashboard,
    Log,
    State,
    KillConfirm,
}

pub struct App {
    pub repos: Vec<RepoInfo>,
    pub selected: usize,
    pub mode: Mode,
    pub log_scroll: usize,
    pub config: Config,
    pub last_scan: std::time::Instant,
}

impl App {
    pub fn new(config: Config) -> Self {
        let repos = discover_repos(&config);
        App {
            repos,
            selected: 0,
            mode: Mode::Dashboard,
            log_scroll: 0,
            config,
            last_scan: std::time::Instant::now(),
        }
    }

    pub fn refresh(&mut self) {
        self.repos = discover_repos(&self.config);
        self.last_scan = std::time::Instant::now();
        if self.selected >= self.repos.len() && !self.repos.is_empty() {
            self.selected = self.repos.len() - 1;
        }
    }

    pub fn selected_repo(&self) -> Option<&RepoInfo> {
        self.repos.get(self.selected)
    }

    pub fn next(&mut self) {
        if !self.repos.is_empty() {
            self.selected = (self.selected + 1) % self.repos.len();
            self.log_scroll = 0;
        }
    }

    pub fn prev(&mut self) {
        if !self.repos.is_empty() {
            self.selected = if self.selected == 0 {
                self.repos.len() - 1
            } else {
                self.selected - 1
            };
            self.log_scroll = 0;
        }
    }

    pub fn scroll_log_up(&mut self) {
        self.log_scroll = self.log_scroll.saturating_sub(1);
    }

    pub fn scroll_log_down(&mut self) {
        self.log_scroll = self.log_scroll.saturating_add(1);
    }
}
