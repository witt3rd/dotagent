mod app;
mod bus;
mod kill;
mod repo;
mod ui;

use app::{App, Mode};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // parse CLI args
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut repos: Vec<std::path::PathBuf> = Vec::new();
    let mut scan_roots: Vec<std::path::PathBuf> = Vec::new();
    let mut config_path: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--repo" => {
                i += 1;
                if let Some(p) = args.get(i) {
                    repos.push(std::path::PathBuf::from(p));
                }
            }
            "--scan-root" => {
                i += 1;
                if let Some(p) = args.get(i) {
                    scan_roots.push(std::path::PathBuf::from(p));
                }
            }
            "--config" => {
                i += 1;
                if let Some(p) = args.get(i) {
                    config_path = Some(p.clone());
                }
            }
            "-h" | "--help" => {
                println!("dotagent-dash — fleet dashboard for dotagent-inhabited repos");
                println!();
                println!("Usage: dotagent-dash [OPTIONS]");
                println!();
                println!("Options:");
                println!("  --repo PATH         add a repo to monitor (repeatable)");
                println!("  --scan-root PATH    scan a directory for inhabited repos (repeatable)");
                println!("  --config PATH       config file (default: ~/.config/dotagent/dashboard.toml)");
                println!("  -h, --help          show this help");
                println!();
                println!("Config file (TOML):");
                println!("  repos = [\"/path/to/repo\"]");
                println!("  scan_roots = [\"/path/to/scan\"]");
                println!();
                println!("Keybindings (in the TUI):");
                println!("  ↑/↓/j/p    navigate");
                println!("  Enter       view log");
                println!("  s           view state");
                println!("  k           kill agent");
                println!("  r           rescan repos");
                println!("  q           quit");
                return Ok(());
            }
            _ => {}
        }
        i += 1;
    }

    // load config file (CLI repos/scan_roots are additive)
    let config_file = config_path.unwrap_or_else(|| {
        std::env::var("DOTAGENT_DASH_CONFIG").unwrap_or_else(|_| {
            dirs_or_default()
                .join("dashboard.toml")
                .to_string_lossy()
                .to_string()
        })
    });
    let config_str = std::fs::read_to_string(&config_file).unwrap_or_default();
    let mut config: repo::Config = toml::from_str(&config_str).unwrap_or(repo::Config {
        repos: vec![],
        scan_roots: vec![],
    });
    // if any config-modifying flags, persist and exit (no TUI)
    if !repos.is_empty() || !scan_roots.is_empty() {
        if let Some(parent) = std::path::Path::new(&config_file).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // load existing config first, then merge
        let existing = std::fs::read_to_string(&config_file).unwrap_or_default();
        let mut config: repo::Config = toml::from_str(&existing).unwrap_or(repo::Config {
            repos: vec![],
            scan_roots: vec![],
        });
        // add only repos that aren't already present
        for r in &repos {
            if !config.repos.iter().any(|p| p == r) {
                config.repos.push(r.clone());
                println!("added repo: {}", r.display());
            } else {
                println!("repo already tracked: {}", r.display());
            }
        }
        for s in &scan_roots {
            if !config.scan_roots.iter().any(|p| p == s) {
                config.scan_roots.push(s.clone());
                println!("added scan root: {}", s.display());
            } else {
                println!("scan root already tracked: {}", s.display());
            }
        }
        return Ok(());
    }

    // no config-modifying flags: load config and launch TUI
    let config_str = std::fs::read_to_string(&config_file).unwrap_or_default();
    let config: repo::Config = toml::from_str(&config_str).unwrap_or(repo::Config {
        repos: vec![],
        scan_roots: vec![],
    });

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(config);
    app.refresh();

    // setup FIFO bus for real-time notifications from dispatch hooks
    let bus_path = std::env::var("DOTAGENT_BUS").unwrap_or_else(|_| bus::DEFAULT_BUS.to_string());
    let bus = bus::Bus::open(&bus_path);
    app.bus_path = Some(bus_path);

    // event loop
    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        // non-blocking read from bus (instant notification from hooks)
        let bus_events = bus.read_nonblocking();
        for evt in &bus_events {
            // refresh the repo that was notified
            if let Some(repo) = app.repos.iter_mut().find(|r| r.path.to_string_lossy() == evt.repo) {
                *repo = repo::RepoInfo::from_path(&repo.path).unwrap_or_else(|| repo.clone());
            }
            app.last_activity = Some(chrono::Local::now().format("%H:%M:%S").to_string());
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match app.mode {
                    Mode::Dashboard => match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('r') => app.refresh(),
                        KeyCode::Up | KeyCode::Char('p') => app.prev(),
                        KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => app.next(),
                        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                            app.load_log();
                            app.mode = Mode::Log;
                        }
                        KeyCode::Char('s') => app.mode = Mode::State,
                        KeyCode::Char('k') => {
                            if let Some(repo) = app.selected_repo() {
                                if matches!(&repo.lock, crate::repo::LockState::Held { .. }) {
                                    app.mode = Mode::KillConfirm;
                                }
                            }
                        }
                        _ => {}
                    },
                    Mode::Log => match key.code {
                        KeyCode::Esc => app.mode = Mode::Dashboard,
                        KeyCode::Up | KeyCode::Char('p') => app.log_up(),
                        KeyCode::Down | KeyCode::Char('j') => app.log_down(),
                        KeyCode::Enter => app.mode = Mode::LogDetail,
                        KeyCode::Char('q') => break,
                        _ => {}
                    },
                    Mode::LogDetail => match key.code {
                        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => app.mode = Mode::Log,
                        KeyCode::Char('q') => break,
                        _ => {}
                    },
                    Mode::State => match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => app.mode = Mode::Dashboard,
                        _ => {}
                    },
                    Mode::KillConfirm => match key.code {
                        KeyCode::Esc => app.mode = Mode::Dashboard,
                        KeyCode::Char('y') => {
                            if let Some(repo) = app.selected_repo().cloned() {
                                let _ = kill::kill_agent(&repo);
                                app.refresh();
                            }
                            app.mode = Mode::Dashboard;
                        }
                        _ => {}
                    },
                }
            }
        }
    }

    // restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn dirs_or_default() -> std::path::PathBuf {
    std::env::var("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|_| {
            std::env::var("HOME")
                .map(|h| std::path::PathBuf::from(h).join(".config"))
        })
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("dotagent")
}
