use crate::app::{App, Mode};
use crate::repo::{LockState, Status};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Cell, List, ListItem, ListState, Paragraph, Row, Table, Wrap};
use ratatui::Frame;

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(55), // repo list
            Constraint::Percentage(35), // detail panel
            Constraint::Length(1),      // keybar
        ])
        .split(f.area());

    draw_repo_list(f, app, chunks[0]);
    draw_detail(f, app, chunks[1]);
    draw_keybar(f, app, chunks[2]);

    match app.mode {
        Mode::Log => {
            let area = centered_rect(90, 85, f.area());
            f.render_widget(Clear, area);
            draw_log_list(f, app, area);
        }
        Mode::LogDetail => draw_log_detail(f, app),
        Mode::State => {
            let area = centered_rect(80, 70, f.area());
            f.render_widget(Clear, area);
            draw_state_popup(f, app, area);
        }
        Mode::KillConfirm => draw_kill_confirm(f, app),
        Mode::Dashboard => {}
    }
}

fn draw_repo_list(f: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec![
        Cell::from("Repo").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Status").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Lock").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Events").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("In").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Out").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Handoffs").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Chain").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Last").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ]);

    let rows: Vec<Row> = app
        .repos
        .iter()
        .map(|repo| {
            let status_cell = match &repo.status {
                Status::Active => Cell::from("active").style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Status::Idle => Cell::from("idle").style(Style::default().fg(Color::Rgb(120, 120, 120))),
                Status::Stale => Cell::from("stale").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Status::Error => Cell::from("error").style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            };

            let lock_cell = match &repo.lock {
                LockState::Free => Cell::from("free").style(Style::default().fg(Color::Rgb(100, 100, 100))),
                LockState::Held { pid, .. } => Cell::from(format!("held ({})", pid))
                    .style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                LockState::Stale { pid, .. } => Cell::from(format!("STALE ({})", pid))
                    .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            };

            let ev_color = if repo.event_count > 0 { Color::Green } else { Color::Rgb(100, 100, 100) };
            let in_color = if repo.open_inbound > 0 { Color::Yellow } else { Color::Rgb(100, 100, 100) };
            let out_color = Color::Rgb(130, 130, 255);
            let chain_color = if repo.chain_depth > 3 { Color::Red } else if repo.chain_depth > 0 { Color::Yellow } else { Color::Rgb(100, 100, 100) };

            let last_cell = match &repo.last_event {
                Some(ts) => {
                    let age = chrono::Utc::now().signed_duration_since(*ts);
                    if age.num_seconds() < 60 {
                        Cell::from("just now").style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
                    } else if age.num_minutes() < 60 {
                        Cell::from(format!("{}m ago", age.num_minutes())).style(Style::default().fg(Color::Cyan))
                    } else if age.num_hours() < 24 {
                        Cell::from(format!("{}h ago", age.num_hours())).style(Style::default().fg(Color::Rgb(180, 180, 80)))
                    } else {
                        Cell::from(format!("{}d ago", age.num_days()))
                            .style(Style::default().fg(Color::Yellow))
                    }
                }
                None => Cell::from("never").style(Style::default().fg(Color::Rgb(80, 80, 80))),
            };

            Row::new(vec![
                Cell::from(repo.identity.clone()).style(Style::default().fg(Color::White)),
                status_cell,
                lock_cell,
                Cell::from(repo.event_count.to_string()).style(Style::default().fg(ev_color)),
                Cell::from(repo.open_inbound.to_string()).style(Style::default().fg(in_color)),
                Cell::from(repo.open_outbound.to_string()).style(Style::default().fg(out_color)),
                Cell::from(repo.handoffs.to_string()).style(Style::default().fg(Color::Rgb(180, 130, 255))),
                Cell::from(repo.chain_depth.to_string()).style(Style::default().fg(chain_color)),
                last_cell,
            ])
        })
        .collect();

    let widths = [
        Constraint::Min(20),  // Repo
        Constraint::Length(6), // Status
        Constraint::Length(12),// Lock
        Constraint::Length(6), // Events
        Constraint::Length(3), // In
        Constraint::Length(3), // Out
        Constraint::Length(8), // Handoffs
        Constraint::Length(5), // Chain
        Constraint::Min(8),   // Last
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(format!(" dotagent dashboard — {} repos  [r]escan  [q]uit ", app.repos.len()))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 40, 80))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");

    let mut state = ratatui::widgets::TableState::default();
    state.select(Some(app.selected));
    f.render_stateful_widget(table, area, &mut state);
}

fn draw_detail(f: &mut Frame, app: &App, area: Rect) {
    let repo = match app.selected_repo() {
        Some(r) => r,
        None => {
            let empty = Block::default()
                .title(" detail ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray));
            f.render_widget(empty, area);
            return;
        }
    };

    let lock_str = match &repo.lock {
        LockState::Free => "free".to_string(),
        LockState::Held { pid, since } => {
            let age_secs = since.parse::<i64>().unwrap_or(0);
            let now = chrono::Utc::now().timestamp();
            let ago = now - age_secs;
            format!("held (PID {}, {}s)", pid, ago.max(0))
        }
        LockState::Stale { pid, since } => {
            let age_secs = since.parse::<i64>().unwrap_or(0);
            let now = chrono::Utc::now().timestamp();
            let ago = now - age_secs;
            format!("STALE (PID {}, {}s)", pid, ago.max(0))
        }
    };

    let handoff_str = repo.last_handoff_subject.as_deref().unwrap_or("(none)");

    let rows = vec![
        Row::new(vec![
            Cell::from("Repo").style(Style::default().fg(Color::Cyan)),
            Cell::from(format!("{}  ({})", repo.identity, repo.path.display()))
                .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Row::new(vec![
            Cell::from("Events").style(Style::default().fg(Color::Cyan)),
            Cell::from(format!("{}", repo.event_count)).style(Style::default().fg(Color::Green)),
        ]),
        Row::new(vec![
            Cell::from("In").style(Style::default().fg(Color::Cyan)),
            Cell::from(format!("{}", repo.open_inbound))
                .style(if repo.open_inbound > 0 { Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::Rgb(100, 100, 100)) }),
        ]),
        Row::new(vec![
            Cell::from("Out").style(Style::default().fg(Color::Cyan)),
            Cell::from(format!("{}", repo.open_outbound)).style(Style::default().fg(Color::Rgb(130, 130, 255))),
        ]),
        Row::new(vec![
            Cell::from("Handoffs").style(Style::default().fg(Color::Cyan)),
            Cell::from(format!("{}", repo.handoffs)).style(Style::default().fg(Color::Rgb(180, 130, 255))),
        ]),
        Row::new(vec![
            Cell::from("Chain").style(Style::default().fg(Color::Cyan)),
            Cell::from(format!("{}", repo.chain_depth))
                .style(if repo.chain_depth > 3 { Style::default().fg(Color::Red).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::Rgb(100, 100, 100)) }),
        ]),
        Row::new(vec![
            Cell::from("Lock").style(Style::default().fg(Color::Cyan)),
            Cell::from(lock_str.as_str()).style(match &repo.lock {
                LockState::Free => Style::default().fg(Color::Rgb(100, 100, 100)),
                LockState::Held { .. } => Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
                LockState::Stale { .. } => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            }),
        ]),
        Row::new(vec![
            Cell::from("Last").style(Style::default().fg(Color::Cyan)),
            Cell::from(handoff_str).style(Style::default().fg(Color::Rgb(160, 160, 160))),
        ]),
    ];

    let widths = [
        Constraint::Length(8),
        Constraint::Min(40),
    ];

    let table = Table::new(rows, widths)
        .block(
            Block::default()
                .title(format!(" {} — detail ", repo.identity))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        )
        .column_spacing(2);

    f.render_widget(table, area);
}

fn draw_log_list(f: &mut Frame, app: &App, area: Rect) {
    let repo = match app.selected_repo() {
        Some(r) => r,
        None => return,
    };

    let items: Vec<ListItem> = app
        .log_entries
        .iter()
        .map(|entry| {
            let color = match entry.event_type {
                'H' => Color::Cyan,
                'O' => Color::Blue,
                'I' => Color::Green,
                'R' => Color::Yellow,
                'C' => Color::Magenta,
                'S' => Color::DarkGray,
                _ => Color::White,
            };
            let line = Line::from(vec![
                Span::styled(
                    format!("{} ", &entry.filename[..entry.filename.len().min(8)]),
                    Style::default().fg(color),
                ),
                Span::raw(&entry.subject),
            ]);
            ListItem::new(line)
        })
        .collect();

    let block = Block::default()
        .title(format!(" {} — log ({} events, {} selected) ", repo.identity, app.log_entries.len(), app.log_selected + 1))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let mut state = ListState::default();
    state.select(Some(app.log_selected));
    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_log_detail(f: &mut Frame, app: &App) {
    let area = centered_rect(85, 80, f.area());
    f.render_widget(Clear, area);

    let entry = match app.log_entries.get(app.log_selected) {
        Some(e) => e,
        None => return,
    };

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        &entry.filename,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::raw(""));

    for line in entry.body.lines() {
        if line.starts_with("---") {
            lines.push(Line::from(Span::styled(line, Style::default().fg(Color::DarkGray))));
        } else if line.contains(": ") && !line.starts_with(' ') {
            if let Some((key, val)) = line.split_once(": ") {
                lines.push(Line::from(vec![
                    Span::styled(format!("{}:", key), Style::default().fg(Color::Yellow)),
                    Span::raw(format!(" {}", val)),
                ]));
            } else {
                lines.push(Line::raw(line));
            }
        } else {
            lines.push(Line::raw(line));
        }
    }

    let block = Block::default()
        .title(format!(" {} — {} ", entry.event_type, entry.filename))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .scroll((app.detail_scroll as u16, 0))
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

fn draw_keybar(f: &mut Frame, app: &App, area: Rect) {
    let mode_str = match app.mode {
        Mode::Dashboard => "[↑↓] select  [Enter] log  [s] state  [k] kill  [r] rescan  [q] quit",
        Mode::Log => "[↑↓] select  [Enter] view  [Esc] back  [q] quit",
        Mode::LogDetail => "[↑↓] scroll  [h/Esc] back to log  [q] quit",
        Mode::State => "[Esc] back  [q] quit",
        Mode::KillConfirm => "[y] confirm kill  [Esc] cancel",
    };
    let bar = Paragraph::new(Line::from(Span::styled(
        mode_str,
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )));
    f.render_widget(bar, area);
}

fn draw_state_popup(f: &mut Frame, app: &App, area: Rect) {

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
        .title(" kill agent ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));

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
