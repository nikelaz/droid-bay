use std::{
    io,
    path::PathBuf,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::{Duration, Instant},
};

use crossterm::{
    event::{
        self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
    },
    execute,
};
use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Cell, Clear, Paragraph, Row, Table, TableState, Wrap},
    Frame,
};

use super::{apply_skill, status, Action, InstalledStatus, SKILLS};

const BACKGROUND: Color = Color::Rgb(13, 17, 26);
const PANEL: Color = Color::Rgb(20, 26, 38);
const HIGHLIGHT: Color = Color::Rgb(30, 51, 66);
const BORDER: Color = Color::Rgb(49, 62, 82);
const TEXT: Color = Color::Rgb(220, 230, 242);
const MUTED: Color = Color::Rgb(120, 138, 161);
const CYAN: Color = Color::Rgb(98, 226, 211);
const VIOLET: Color = Color::Rgb(181, 164, 250);
const GREEN: Color = Color::Rgb(143, 218, 164);
const AMBER: Color = Color::Rgb(244, 195, 122);
const RED: Color = Color::Rgb(246, 133, 159);
const MIN_WIDTH: u16 = 64;
const MIN_HEIGHT: u16 = 22;
const DETAIL_WIDTH: u16 = 108;

#[derive(Clone, Copy, PartialEq)]
enum Focus {
    Skills,
    Locations,
    Actions,
}

#[derive(Clone, Copy)]
enum Dialog {
    Remove,
    Activity(usize),
}

enum Progress {
    Started(&'static str),
    Finished(&'static str, Result<(), String>),
}

struct Job {
    receiver: Receiver<Progress>,
    started: Instant,
    action: Action,
    total: usize,
    completed: usize,
    errors: usize,
    current: &'static str,
}

struct App {
    root: PathBuf,
    selected: Vec<bool>,
    statuses: Vec<[InstalledStatus; 2]>,
    table: TableState,
    focus: Focus,
    location: usize,
    locations: [bool; 2],
    action: usize,
    dialog: Option<Dialog>,
    job: Option<Job>,
    activity: Vec<(String, Color)>,
    message: String,
    message_color: Color,
    quit_after_job: bool,
}

#[derive(Default)]
struct Areas {
    skills: Rect,
    locations: [Rect; 2],
    actions: [Rect; 2],
    confirm: Rect,
}

fn refresh(app: &mut App) {
    app.statuses = SKILLS
        .iter()
        .map(|skill| {
            [
                status(&app.root.join(".agents/skills").join(skill.name)),
                status(&app.root.join(".claude/skills").join(skill.name)),
            ]
        })
        .collect();
}

fn action_name(action: Action) -> &'static str {
    match action {
        Action::Install => "Install",
        Action::Update => "Update",
        Action::InstallAndUpdate => "Install & Update",
        Action::Remove => "Uninstall",
    }
}

fn available_actions(app: &App) -> [Action; 2] {
    let mut installed = false;
    let mut missing = false;
    for (index, selected) in app.selected.iter().enumerate() {
        if !selected {
            continue;
        }
        for (location, enabled) in app.locations.iter().enumerate() {
            if !enabled {
                continue;
            }
            if matches!(
                app.statuses[index][location],
                InstalledStatus::Installed { .. }
            ) {
                installed = true;
            } else {
                missing = true;
            }
        }
    }
    let primary = match (installed, missing) {
        (true, true) => Action::InstallAndUpdate,
        (true, false) => Action::Update,
        _ => Action::Install,
    };
    [primary, Action::Remove]
}

fn badge(status: &InstalledStatus, version: &str) -> (String, Color) {
    match status {
        InstalledStatus::Absent => ("—".into(), MUTED),
        InstalledStatus::Error(_) => ("Error".into(), RED),
        InstalledStatus::Installed {
            version: installed,
            linked,
        } => {
            let (label, color) = match installed {
                Some(installed) if installed == version => (format!("{installed} ✓"), GREEN),
                Some(installed) => (format!("{installed} ↑"), AMBER),
                None => ("Unknown".into(), AMBER),
            };
            (format!("{label}{}", if *linked { " ↗" } else { "" }), color)
        }
    }
}

fn panel(title: &str, active: bool) -> Block<'_> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(format!(" {}{title} ", if active { "› " } else { "" }))
        .title_style(Style::default().fg(if active { CYAN } else { MUTED }))
        .border_style(Style::default().fg(if active { CYAN } else { BORDER }))
        .style(Style::default().bg(PANEL).fg(TEXT))
}

fn center(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

fn description(index: usize) -> &'static str {
    SKILLS[index]
        .files
        .iter()
        .find(|(name, _)| *name == "SKILL.md")
        .and_then(|(_, bytes)| std::str::from_utf8(bytes).ok())
        .and_then(|content| {
            content
                .lines()
                .find_map(|line| line.strip_prefix("description:"))
        })
        .map(str::trim)
        .unwrap_or("No description available.")
}

fn render(frame: &mut Frame, app: &mut App) -> Areas {
    let screen = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(BACKGROUND).fg(TEXT)),
        screen,
    );
    if screen.width < MIN_WIDTH || screen.height < MIN_HEIGHT {
        frame.render_widget(Paragraph::new("DROID BAY\nSETUP\n\nEnlarge the terminal to at least 64 × 22.\nYour selection is preserved.  Q to quit.")
            .style(Style::default().fg(VIOLET)).wrap(Wrap { trim: true }), screen.inner(Margin::new(2, 1)));
        return Areas::default();
    }
    let content = screen.inner(Margin::new(2, 1));
    let sections = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(6),
        Constraint::Length(5),
        Constraint::Length(3),
        Constraint::Length(2),
    ])
    .split(content);
    let count = app.selected.iter().filter(|value| **value).count();
    let header = vec![
        Line::styled(
            "DROID BAY",
            Style::default().fg(VIOLET).add_modifier(Modifier::BOLD),
        ),
        Line::styled("SETUP", Style::default().fg(MUTED)),
        Line::from(vec![
            Span::styled(format!("{count:02} selected"), Style::default().fg(CYAN)),
            Span::styled(
                format!(
                    "  /  {:02} available    •    {}",
                    SKILLS.len(),
                    app.root.display()
                ),
                Style::default().fg(MUTED),
            ),
        ]),
    ];
    frame.render_widget(Paragraph::new(header), sections[0]);
    let body = if screen.width >= DETAIL_WIDTH {
        Layout::horizontal([Constraint::Percentage(62), Constraint::Percentage(38)])
            .spacing(1)
            .split(sections[1])
    } else {
        Layout::horizontal([Constraint::Percentage(100)]).split(sections[1])
    };
    let mut areas = Areas {
        skills: body[0],
        ..Areas::default()
    };
    let rows = SKILLS.iter().enumerate().map(|(index, skill)| {
        let agent = badge(&app.statuses[index][0], skill.version);
        let claude = badge(&app.statuses[index][1], skill.version);
        Row::new(vec![
            Cell::from(if app.selected[index] { "●" } else { "○" })
                .style(Style::default().fg(if app.selected[index] { CYAN } else { MUTED })),
            Cell::from(skill.name).style(Style::default().fg(TEXT)),
            Cell::from(skill.version).style(Style::default().fg(VIOLET)),
            Cell::from(agent.0).style(Style::default().fg(agent.1)),
            Cell::from(claude.0).style(Style::default().fg(claude.1)),
        ])
    });
    let table = Table::new(
        rows,
        [
            Constraint::Length(1),
            Constraint::Min(20),
            Constraint::Length(7),
            Constraint::Length(12),
            Constraint::Length(12),
        ],
    )
    .header(
        Row::new(["", "SKILL", "BUNDLE", ".AGENTS", ".CLAUDE"])
            .style(Style::default().fg(MUTED))
            .bottom_margin(1),
    )
    .block(
        panel("01 / Skills", app.focus == Focus::Skills).title_bottom(
            Line::styled(
                format!(
                    " {}/{} · Space select · A all ",
                    app.table.selected().map_or(0, |index| index + 1),
                    SKILLS.len()
                ),
                Style::default().fg(MUTED),
            )
            .right_aligned(),
        ),
    )
    .column_spacing(1)
    .highlight_symbol("› ")
    .row_highlight_style(Style::default().bg(HIGHLIGHT).add_modifier(Modifier::BOLD));
    frame.render_stateful_widget(table, areas.skills, &mut app.table);
    if body.len() > 1 {
        let index = app.table.selected().unwrap_or(0);
        if let Some(skill) = SKILLS.get(index) {
            let mut detail = vec![
                Line::styled(
                    skill.name,
                    Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
                ),
                Line::styled(
                    format!("Bundled version {}", skill.version),
                    Style::default().fg(VIOLET),
                ),
                Line::default(),
                Line::from(description(index)),
                Line::default(),
            ];
            for (name, state) in [".agents", ".claude"].iter().zip(&app.statuses[index]) {
                let (label, color) = badge(state, skill.version);
                detail.push(Line::styled(
                    format!("{name}: {label}"),
                    Style::default().fg(color),
                ));
                if let InstalledStatus::Error(error) = state {
                    detail.push(Line::styled(error.as_str(), Style::default().fg(RED)));
                }
            }
            detail.push(Line::default());
            detail.push(Line::styled(
                "✓ current  ↑ different  ↗ linked",
                Style::default().fg(MUTED),
            ));
            frame.render_widget(
                Paragraph::new(detail)
                    .wrap(Wrap { trim: true })
                    .block(panel("Skill brief", false)),
                body[1],
            );
        }
    }
    frame.render_widget(
        Block::default().style(Style::default().bg(PANEL)),
        sections[2],
    );
    let locations = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .spacing(1)
        .split(sections[2]);
    for index in 0..2 {
        areas.locations[index] = locations[index];
        let active = app.focus == Focus::Locations && app.location == index;
        let title = if index == 0 {
            "02 / All harnesses [G]"
        } else {
            "Claude Code [C]"
        };
        let path = if index == 0 {
            ".agents/skills"
        } else {
            ".claude/skills"
        };
        let mode = if index == 0 {
            ""
        } else if app.locations[0] && app.locations[1] {
            "↗ Linked to .agents"
        } else {
            "Independent skill copies"
        };
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(vec![
                    Span::styled(
                        if app.locations[index] {
                            "●  "
                        } else {
                            "○  "
                        },
                        Style::default().fg(CYAN),
                    ),
                    Span::styled(
                        path,
                        Style::default().fg(if app.locations[index] { TEXT } else { MUTED }),
                    ),
                ]),
                Line::styled(
                    mode,
                    Style::default().fg(if index == 1 && app.locations[0] && app.locations[1] {
                        VIOLET
                    } else {
                        MUTED
                    }),
                ),
            ])
            .block(panel(title, active)),
            locations[index],
        );
    }
    frame.render_widget(
        Block::default().style(Style::default().bg(PANEL)),
        sections[3],
    );
    let buttons = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .spacing(1)
        .split(sections[3]);
    for (index, action) in available_actions(app).iter().enumerate() {
        let color = match action {
            Action::Install | Action::InstallAndUpdate => CYAN,
            Action::Update => VIOLET,
            Action::Remove => RED,
        };
        let label = format!(
            "{} [{}]",
            action_name(*action),
            if index == 0 { "I" } else { "R" }
        );
        areas.actions[index] = buttons[index];
        let active = app.focus == Focus::Actions && app.action == index;
        frame.render_widget(
            Paragraph::new(format!("{}{label}", if active { "› " } else { "" }))
                .centered()
                .style(
                    Style::default()
                        .fg(if app.job.is_some() { MUTED } else { color })
                        .add_modifier(Modifier::BOLD),
                )
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(if active { color } else { BORDER }))
                        .style(Style::default().bg(if active { HIGHLIGHT } else { PANEL })),
                ),
            buttons[index],
        );
    }
    let message = if let Some(job) = &app.job {
        const SPINNER: [&str; 4] = ["◐", "◓", "◑", "◒"];
        format!(
            "{} {}  {}/{}  {}",
            SPINNER[(job.started.elapsed().as_millis() / 120 % 4) as usize],
            action_name(job.action),
            job.completed,
            job.total,
            job.current
        )
    } else {
        app.message.clone()
    };
    let help = if app.job.is_some() {
        if app.quit_after_job {
            "Finishing the operation before exiting…"
        } else {
            "Working…  L activity  Q finish operation and exit"
        }
    } else {
        match app.focus {
            Focus::Skills => "↑↓ move  Space select  A all  Tab next  L log  Q quit",
            Focus::Locations => "←→ choose  Space toggle  Tab next  G/C toggle  Q quit",
            Focus::Actions => "←→ choose  Enter run  Tab next  I/R act  L log  Q quit",
        }
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(
                message,
                Style::default().fg(if app.job.is_some() {
                    CYAN
                } else {
                    app.message_color
                }),
            ),
            Line::styled(help, Style::default().fg(MUTED)),
        ]),
        sections[4],
    );
    if let Some(dialog) = app.dialog {
        let popup = match dialog {
            Dialog::Remove => center(screen, 58, 11),
            Dialog::Activity(_) => center(screen, 90, screen.height.saturating_sub(4)),
        };
        frame.render_widget(Clear, popup);
        match dialog {
            Dialog::Remove => {
                let destinations = match app.locations {
                    [true, true] => ".agents and .claude",
                    [true, false] => ".agents",
                    _ => ".claude",
                };
                let lines = vec![
                    Line::styled(
                        "Uninstall selected skills?",
                        Style::default().fg(RED).add_modifier(Modifier::BOLD),
                    ),
                    Line::default(),
                    Line::from(format!("{count} skills from {destinations}.")),
                    Line::from("Installed folders and their local edits will be deleted."),
                    Line::from("Links are removed without deleting their targets."),
                    Line::default(),
                    Line::styled(
                        "Enter / Y  uninstall    Esc / N  keep skills",
                        Style::default().fg(AMBER),
                    ),
                ];
                frame.render_widget(
                    Paragraph::new(lines)
                        .wrap(Wrap { trim: true })
                        .block(panel("Confirm uninstall", true)),
                    popup,
                );
                areas.confirm = Rect::new(
                    popup.x + 1,
                    popup.y + 7,
                    popup.width.saturating_sub(2) / 2,
                    1,
                );
            }
            Dialog::Activity(offset) => {
                let visible = popup.height.saturating_sub(2) as usize;
                let lines = if app.activity.is_empty() {
                    vec![Line::styled(
                        "No operations yet. Select skills and install when ready.",
                        Style::default().fg(MUTED),
                    )]
                } else {
                    let mut lines = Vec::new();
                    let width = popup.width.saturating_sub(2) as usize;
                    for (text, color) in &app.activity {
                        let mut content = String::new();
                        let mut used = 0;
                        for character in text.chars() {
                            let size = Span::raw(character.to_string()).width();
                            if character == '\n' || used + size > width {
                                lines.push(Line::styled(
                                    std::mem::take(&mut content),
                                    Style::default().fg(*color),
                                ));
                                used = 0;
                            }
                            if character != '\n' {
                                content.push(character);
                                used += size;
                            }
                        }
                        lines.push(Line::styled(content, Style::default().fg(*color)));
                    }
                    lines
                };
                let offset = offset.min(lines.len().saturating_sub(visible));
                app.dialog = Some(Dialog::Activity(offset));
                frame.render_widget(
                    Paragraph::new(
                        lines
                            .into_iter()
                            .skip(offset)
                            .take(visible)
                            .collect::<Vec<_>>(),
                    )
                    .block(panel("Activity / ↑↓ scroll / Esc close", true)),
                    popup,
                );
            }
        }
    }
    areas
}

fn start_action(app: &mut App, action: Action) {
    if app.job.is_some() {
        return;
    }
    let indices: Vec<usize> = app
        .selected
        .iter()
        .enumerate()
        .filter_map(|(index, selected)| selected.then_some(index))
        .collect();
    if indices.is_empty() || !app.locations.iter().any(|selected| *selected) {
        app.message = "Select at least one skill and one destination.".into();
        app.message_color = AMBER;
        return;
    }
    if action == Action::Remove && !matches!(app.dialog, Some(Dialog::Remove)) {
        app.dialog = Some(Dialog::Remove);
        return;
    }
    app.dialog = None;
    let root = app.root.clone();
    let locations = app.locations;
    let total = indices.len();
    let (sender, receiver) = mpsc::channel();
    app.activity.push((
        format!("{} / {total} selected skills", action_name(action)),
        VIOLET,
    ));
    app.job = Some(Job {
        receiver,
        started: Instant::now(),
        action,
        total,
        completed: 0,
        errors: 0,
        current: "Preparing…",
    });
    thread::spawn(move || {
        for index in indices {
            let skill = &SKILLS[index];
            if sender.send(Progress::Started(skill.name)).is_err() {
                break;
            }
            let result = apply_skill(skill, &root, action, locations[0], locations[1])
                .map_err(|error| error.to_string());
            if sender.send(Progress::Finished(skill.name, result)).is_err() {
                break;
            }
        }
    });
}

fn receive_progress(app: &mut App) {
    let Some(job) = &mut app.job else {
        return;
    };
    loop {
        match job.receiver.try_recv() {
            Ok(Progress::Started(name)) => job.current = name,
            Ok(Progress::Finished(name, result)) => {
                job.completed += 1;
                match result {
                    Ok(()) => app.activity.push((format!("✓ {name}"), GREEN)),
                    Err(error) => {
                        job.errors += 1;
                        app.activity.push((format!("× {name}: {error}"), RED));
                    }
                }
            }
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => break,
        }
    }
    app.message = format!(
        "{} complete · {}/{} processed · {} errors · L for details",
        action_name(job.action),
        job.completed,
        job.total,
        job.errors
    );
    app.message_color = if job.errors == 0 && job.completed == job.total {
        GREEN
    } else {
        RED
    };
    app.job = None;
    refresh(app);
}

fn move_skill(app: &mut App, delta: isize) {
    if SKILLS.is_empty() {
        return;
    }
    let index = app.table.selected().unwrap_or(0);
    app.table.select(Some(
        index.saturating_add_signed(delta).min(SKILLS.len() - 1),
    ));
}

fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    if key.kind == KeyEventKind::Release {
        return false;
    }
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.quit_after_job = true;
        return app.job.is_none();
    }
    if let Some(dialog) = &mut app.dialog {
        match dialog {
            Dialog::Remove => match key.code {
                KeyCode::Enter | KeyCode::Char('y') => start_action(app, Action::Remove),
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('q') => app.dialog = None,
                _ => {}
            },
            Dialog::Activity(offset) => match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('l') => app.dialog = None,
                KeyCode::Up | KeyCode::Char('k') => *offset = offset.saturating_sub(1),
                KeyCode::Down | KeyCode::Char('j') => *offset = offset.saturating_add(1),
                KeyCode::Home => *offset = 0,
                KeyCode::End => *offset = usize::MAX,
                KeyCode::PageUp => *offset = offset.saturating_sub(10),
                KeyCode::PageDown => *offset = offset.saturating_add(10),
                _ => {}
            },
        }
        return false;
    }
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.quit_after_job = true;
            return app.job.is_none();
        }
        KeyCode::Char('l') => app.dialog = Some(Dialog::Activity(0)),
        _ if app.job.is_some() => {}
        KeyCode::Tab | KeyCode::BackTab => {
            let reverse =
                key.code == KeyCode::BackTab || key.modifiers.contains(KeyModifiers::SHIFT);
            app.focus = match (app.focus, reverse) {
                (Focus::Skills, false) | (Focus::Actions, true) => Focus::Locations,
                (Focus::Locations, false) | (Focus::Skills, true) => Focus::Actions,
                _ => Focus::Skills,
            };
        }
        KeyCode::Up | KeyCode::Char('k') if app.focus == Focus::Skills => move_skill(app, -1),
        KeyCode::Down | KeyCode::Char('j') if app.focus == Focus::Skills => move_skill(app, 1),
        KeyCode::Home if app.focus == Focus::Skills => app.table.select(Some(0)),
        KeyCode::End if app.focus == Focus::Skills => {
            app.table.select(Some(SKILLS.len().saturating_sub(1)))
        }
        KeyCode::Left | KeyCode::Right => match app.focus {
            Focus::Locations => app.location = 1 - app.location,
            Focus::Actions => {
                app.action = 1 - app.action;
            }
            Focus::Skills => {}
        },
        KeyCode::Enter | KeyCode::Char(' ') => match app.focus {
            Focus::Skills => {
                if let Some(index) = app.table.selected() {
                    app.selected[index] = !app.selected[index];
                }
            }
            Focus::Locations => app.locations[app.location] = !app.locations[app.location],
            Focus::Actions => {
                let action = available_actions(app)[app.action];
                start_action(app, action);
            }
        },
        KeyCode::Char('a') => {
            let select = !app.selected.iter().all(|value| *value);
            app.selected.fill(select);
        }
        KeyCode::Char('g') => app.locations[0] = !app.locations[0],
        KeyCode::Char('c') => app.locations[1] = !app.locations[1],
        KeyCode::Char('i') | KeyCode::Char('u') => {
            let action = available_actions(app)[0];
            start_action(app, action);
        }
        KeyCode::Char('r') => start_action(app, Action::Remove),
        KeyCode::F(5) => {
            refresh(app);
            app.message = "Installation status refreshed.".into();
            app.message_color = CYAN;
        }
        _ => {}
    }
    false
}

fn contains(area: Rect, x: u16, y: u16) -> bool {
    x >= area.x && x < area.right() && y >= area.y && y < area.bottom()
}

pub(super) fn run(root: PathBuf) -> io::Result<()> {
    let mut app = App {
        root,
        selected: vec![false; SKILLS.len()],
        statuses: Vec::new(),
        table: TableState::default().with_selected(if SKILLS.is_empty() { None } else { Some(0) }),
        focus: Focus::Skills,
        location: 0,
        locations: [true, false],
        action: 0,
        dialog: None,
        job: None,
        activity: Vec::new(),
        message: "Pick your skills. Space to select, Tab to choose destinations.".into(),
        message_color: MUTED,
        quit_after_job: false,
    };
    refresh(&mut app);
    ratatui::run(|terminal| {
        execute!(io::stdout(), event::EnableMouseCapture)?;
        let result = (|| loop {
            receive_progress(&mut app);
            if app.quit_after_job && app.job.is_none() {
                return Ok(());
            }
            let mut areas = Areas::default();
            terminal.draw(|frame| areas = render(frame, &mut app))?;
            let timeout = if app.job.is_some() {
                Duration::from_millis(80)
            } else {
                Duration::from_millis(250)
            };
            if !event::poll(timeout)? {
                continue;
            }
            match event::read()? {
                Event::Key(key) => {
                    if handle_key(&mut app, key) {
                        return Ok(());
                    }
                }
                Event::Mouse(mouse) => {
                    if let Some(Dialog::Activity(offset)) = &mut app.dialog {
                        match mouse.kind {
                            MouseEventKind::ScrollUp => *offset = offset.saturating_sub(1),
                            MouseEventKind::ScrollDown => *offset = offset.saturating_add(1),
                            _ => {}
                        }
                    } else if matches!(app.dialog, Some(Dialog::Remove)) {
                        if mouse.kind == MouseEventKind::Down(MouseButton::Left)
                            && contains(areas.confirm, mouse.column, mouse.row)
                        {
                            start_action(&mut app, Action::Remove);
                        }
                    } else if app.job.is_none() {
                        match mouse.kind {
                            MouseEventKind::ScrollUp
                                if contains(areas.skills, mouse.column, mouse.row) =>
                            {
                                move_skill(&mut app, -1)
                            }
                            MouseEventKind::ScrollDown
                                if contains(areas.skills, mouse.column, mouse.row) =>
                            {
                                move_skill(&mut app, 1)
                            }
                            MouseEventKind::Down(MouseButton::Left) => {
                                if contains(areas.skills, mouse.column, mouse.row)
                                    && mouse.row >= areas.skills.y + 3
                                    && mouse.row < areas.skills.bottom() - 1
                                {
                                    let index = app.table.offset()
                                        + (mouse.row - areas.skills.y - 3) as usize;
                                    if index < SKILLS.len() {
                                        app.focus = Focus::Skills;
                                        app.table.select(Some(index));
                                        app.selected[index] = !app.selected[index];
                                    }
                                }
                                for index in 0..2 {
                                    if contains(areas.locations[index], mouse.column, mouse.row) {
                                        app.focus = Focus::Locations;
                                        app.location = index;
                                        app.locations[index] = !app.locations[index];
                                    }
                                }
                                for (index, action) in available_actions(&app).iter().enumerate() {
                                    if contains(areas.actions[index], mouse.column, mouse.row) {
                                        app.focus = Focus::Actions;
                                        app.action = index;
                                        start_action(&mut app, *action);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        })();
        let cleanup = execute!(io::stdout(), event::DisableMouseCapture);
        result.and(cleanup)
    })
}
