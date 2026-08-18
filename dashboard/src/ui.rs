use crate::app::{App, Mode};
use crate::repo::{LockState, Status};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;
use ratatui::text::Text;

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),    // repo list
            Constraint::Length(1), // keybar
        ])
        .split(f.area());

    draw_repo_list(f, app, chunks[0]);
    draw_keybar(f, app, chunks[1]);

    if app.mode == Mode::Log {
        draw_log_popup(f, app);
    } else if app.mode == Mode::State {
        draw_state_popup(f, app);
    } else if app.mode == Mode::KillConfirm {
        draw_kill_confirm(f, app);
    }
}

fn draw_repo_list(f: &mut Frame, app: &App, area: Rect) {
    let header = Line::from(vec![
        Span::styled("dotagent dashboard", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(
            format!("{} repos  [r]escan  [q]uit", app.repos.len()),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let items: Vec<ListItem> = app
        .repos
        .iter()
        .enumerate()
        .map(|(i, repo)| {
            let status_str = match &repo.status {
                Status::Active => Span::styled("active", Style::default().fg(Color::Green)),
                Status::Idle => Span::styled("idle", Style::default().fg(Color::DarkGray)),
                Status::Stale => Span::styled("stale", Style::default().fg(Color::Yellow)),
                Status::Error => Span::styled("error", Style::default().fg(Color::Red)),
            };

            let lock_str = match &repo.lock {
                LockState::Free => Span::raw("free"),
                LockState::Held { pid, .. } => Span::styled(
                    format!("held ({})", pid),
                    Style::default().fg(Color::Yellow),
                ),
                LockState::Stale { pid, .. } => Span::styled(
                    format!("STALE ({})", pid),
                    Style::default().fg(Color::Red),
                ),
            };

            let last_str = match &repo.last_event {
                Some(ts) => {
                    let age = chrono::Utc::now().signed_duration_since(*ts);
                    if age.num_seconds() < 60 {
                        Span::styled("just now", Style::default().fg(Color::Green))
                    } else if age.num_minutes() < 60 {
                        Span::raw(format!("{}m ago", age.num_minutes()))
                    } else if age.num_hours() < 24 {
                        Span::raw(format!("{}h ago", age.num_hours()))
                    } else {
                        Span::styled(
                            format!("{}d ago", age.num_days()),
                            Style::default().fg(Color::Yellow),
                        )
                    }
                }
                None => Span::styled("never", Style::default().fg(Color::DarkGray)),
            };

            let line = Line::from(vec![
                Span::raw(if i == app.selected { "> " } else { "  " }),
                Span::styled(
                    &repo.identity,
                    Style::default().fg(Color::White).add_modifier(if i == app.selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
                ),
                Span::raw("  "),
                status_str,
                Span::raw("    "),
                lock_str,
                Span::raw(format!(
                    "    {}ev  {} in  {} out  ",
                    repo.event_count, repo.open_inbound, repo.open_outbound
                )),
                last_str,
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(header)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    let mut state = ListState::default();
    state.select(Some(app.selected));
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_keybar(f: &mut Frame, app: &App, area: Rect) {
    let mode_str = match app.mode {
        Mode::Dashboard => "[↑↓] select  [Enter] log  [s] state  [k] kill  [r] rescan  [q] quit",
        Mode::Log => "[↑↓] scroll  [Esc] back  [q] quit",
        Mode::State => "[Esc] back  [q] quit",
        Mode::KillConfirm => "[y] confirm kill  [Esc] cancel",
    };
    let bar = Paragraph::new(Line::from(Span::styled(
        mode_str,
        Style::default().fg(Color::DarkGray),
    )));
    f.render_widget(bar, area);
}

fn draw_log_popup(f: &mut Frame, app: &App) {
    let area = centered_rect(80, 70, f.area());
    f.render_widget(Clear, area);

    let repo = match app.selected_repo() {
        Some(r) => r,
        None => return,
    };

    let log_dir = repo.path.join(".agent/log");
    let mut lines: Vec<Line> = Vec::new();

    if log_dir.exists() {
        let mut files: Vec<String> = std::fs::read_dir(&log_dir)
            .ok()
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .filter(|f| f.ends_with(".md"))
                    .collect()
            })
            .unwrap_or_default();
        files.sort();

        for f in &files {
            let prefix = &f[..1.min(f.len())];
            let color = match prefix {
                "H" => Color::Cyan,
                "O" => Color::Blue,
                "I" => Color::Green,
                "R" => Color::Yellow,
                "C" => Color::Magenta,
                "S" => Color::DarkGray,
                _ => Color::White,
            };
            let content = std::fs::read_to_string(log_dir.join(f))
                .unwrap_or_default();
            let subject = content
                .lines()
                .find(|l| l.starts_with("subject:"))
                .map(|l| l.trim_start_matches("subject:").trim().to_string())
                .unwrap_or_default();

            lines.push(Line::from(vec![
                Span::styled(
                    format!("{} ", &f[..f.len().min(8)]),
                    Style::default().fg(color),
                ),
                Span::raw(subject),
            ]));
        }
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (no events)",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let block = Block::default()
        .title(format!(" {} — log ", repo.identity))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .scroll((app.log_scroll as u16, 0))
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

fn draw_state_popup(f: &mut Frame, app: &App) {
    let area = centered_rect(80, 70, f.area());
    f.render_widget(Clear, area);

    let repo = match app.selected_repo() {
        Some(r) => r,
        None => return,
    };

    let state_file = repo.path.join(".agent/log");
    let mut content = String::new();

    // show latest S event
    if state_file.exists() {
        let mut s_files: Vec<String> = std::fs::read_dir(&state_file)
            .ok()
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .filter(|f| f.starts_with("S--"))
                    .collect()
            })
            .unwrap_or_default();
        s_files.sort();
        if let Some(latest) = s_files.last() {
            content = std::fs::read_to_string(state_file.join(latest)).unwrap_or_default();
        }
    }

    if content.is_empty() {
        content = "  (no state snapshot yet)".to_string();
    }

    let block = Block::default()
        .title(format!(" {} — state ", repo.identity))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(content).block(block).wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

fn draw_kill_confirm(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 20, f.area());
    f.render_widget(Clear, area);

    let repo = match app.selected_repo() {
        Some(r) => r,
        None => return,
    };

    let pid = match &repo.lock {
        LockState::Held { pid, .. } => pid.to_string(),
        _ => return,
    };

    let lines = vec![
        Line::from(Span::styled(
            " Kill agent?",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::from(vec![
            Span::raw("  Repo: "),
            Span::styled(&repo.identity, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::raw("  PID:  "),
            Span::styled(&pid, Style::default().fg(Color::Red)),
        ]),
        Line::raw(""),
        Line::from(Span::styled(
            "  [y] confirm   [Esc] cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
