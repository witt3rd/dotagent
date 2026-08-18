mod app;
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
    // load config
    let config_path = std::env::var("DOTAGENT_DASH_CONFIG")
        .unwrap_or_else(|_| {
            dirs_or_default()
                .join("dashboard.toml")
                .to_string_lossy()
                .to_string()
        });
    let config_str = std::fs::read_to_string(&config_path).unwrap_or_default();
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

    // event loop
    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        if event::poll(Duration::from_millis(200))? {
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
                        KeyCode::Enter | KeyCode::Right => app.mode = Mode::Log,
                        KeyCode::Char('s') => app.mode = Mode::State,
                        KeyCode::Char('k') => {
                            if let Some(repo) = app.selected_repo() {
                                match &repo.lock {
                                    crate::repo::LockState::Held { .. } => {
                                        app.mode = Mode::KillConfirm
                                    }
                                    _ => {} // no agent to kill
                                }
                            }
                        }
                        _ => {}
                    },
                    Mode::Log => match key.code {
                        KeyCode::Esc => app.mode = Mode::Dashboard,
                        KeyCode::Up | KeyCode::Char('p') => app.scroll_log_up(),
                        KeyCode::Down | KeyCode::Char('j') => app.scroll_log_down(),
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
                                match kill::kill_agent(&repo) {
                                    Ok(msg) => {
                                        app.refresh();
                                        // could show a status message
                                        let _ = msg;
                                    }
                                    Err(e) => {
                                        // could show error
                                        let _ = e;
                                    }
                                }
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
