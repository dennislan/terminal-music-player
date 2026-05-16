use crate::app::{App, InputField, PlayMode, Screen};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Wrap};

pub fn draw(frame: &mut Frame, app: &App) {
    match app.screen {
        Screen::Playlist => draw_playlist(frame, app),
        Screen::AddSong => draw_add_song(frame, app),
        Screen::EditAlias => draw_edit_alias(frame, app),
        Screen::ConfirmDelete => {
            draw_playlist(frame, app);
            draw_confirm_delete(frame, app);
        }
        Screen::Error => {
            draw_playlist(frame, app);
            draw_error_dialog(frame, app);
        }
    }
}

fn format_time(secs: f64) -> String {
    if secs <= 0.0 {
        return "00:00".to_string();
    }
    let total_secs = secs as u64;
    let h = total_secs / 3600;
    let m = (total_secs % 3600) / 60;
    let s = total_secs % 60;
    if h > 0 {
        format!("{:02}:{:02}:{:02}", h, m, s)
    } else {
        format!("{:02}:{:02}", m, s)
    }
}

fn draw_playlist(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .split(area);

    let items: Vec<ListItem> = app
        .playlist
        .songs
        .iter()
        .enumerate()
        .map(|(i, song)| {
            let is_selected = i == app.selected_index;
            let is_current = app.current_song.as_deref() == Some(&song.alias);

            let prefix = if is_current {
                if app.is_paused {
                    " \u{23F8} "
                } else {
                    " \u{25B6} "
                }
            } else if is_selected {
                " \u{203A} "
            } else {
                "   "
            };

            let num = i + 1;
            let content = Line::raw(format!("{}{:>3}. {}", prefix, num, song.alias));

            if is_current {
                ListItem::new(content).style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
            } else if is_selected {
                ListItem::new(content).style(
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                ListItem::new(content)
            }
        })
        .collect();

    let content_height = (chunks[0].height.saturating_sub(2)).max(1) as usize;
    let scroll_offset = if app.selected_index >= content_height {
        app.selected_index - content_height + 1
    } else {
        0
    };

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(" Playlist "));
    let mut list_state = ListState::default()
        .with_offset(scroll_offset)
        .with_selected(Some(app.selected_index));
    frame.render_stateful_widget(list, chunks[0], &mut list_state);

    let has_progress = app.current_song.is_some();

    let status_chunks = Layout::horizontal([
        Constraint::Min(10),
        Constraint::Min(10),
        Constraint::Length(8),
    ])
    .split(chunks[1]);

    let status_text = if app.number_input.is_some() {
        app.status_message.clone().unwrap_or_default()
    } else if app.status_restore.is_some() {
        app.status_message.clone().unwrap_or_default()
    } else if app.is_buffering {
        if let Some(alias) = &app.current_song {
            format!("Downloading: {} ...", alias)
        } else {
            "Downloading...".to_string()
        }
    } else {
        match (&app.current_song, &app.is_paused) {
            (Some(alias), false) => format!("Now playing: {}", alias),
            (Some(alias), true) => format!("Paused: {}", alias),
            (None, _) => match &app.status_message {
                Some(msg) => msg.clone(),
                None => "No song playing. Press [a] to add a URL with music inside.".to_string(),
            },
        }
    };

    let is_error = app
        .status_message
        .as_ref()
        .map_or(false, |s| s.starts_with("Error:"));

    let status_style = if is_error {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else if app.is_buffering {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let status_bar = Paragraph::new(Line::raw(status_text))
        .style(status_style)
        .block(Block::default().borders(Borders::ALL).title(" Status "))
        .wrap(Wrap { trim: true });
    frame.render_widget(status_bar, status_chunks[0]);

    if has_progress && app.is_buffering {
        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title(" Progress "))
            .gauge_style(Style::default().fg(Color::Yellow))
            .ratio(0.0)
            .label(Span::styled(
                " Buffering... ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ));
        frame.render_widget(gauge, status_chunks[1]);
    } else if has_progress {
        let (ratio, time_str) = if app.total_duration > 0.0 {
            let r = (app.current_position / app.total_duration).clamp(0.0, 1.0);
            let label = format!(
                " {} / {} ",
                format_time(app.current_position),
                format_time(app.total_duration),
            );
            (r, label)
        } else {
            let label = format!(" {} (live) ", format_time(app.current_position));
            (0.0, label)
        };
        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title(" Progress "))
            .gauge_style(Style::default().fg(Color::Gray))
            .ratio(ratio)
            .label(Span::styled(
                time_str,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ));
        frame.render_widget(gauge, status_chunks[1]);
    } else {
        let empty = Paragraph::new(Line::raw(""))
            .block(Block::default().borders(Borders::ALL).title(" Progress "));
        frame.render_widget(empty, status_chunks[1]);
    }

    let mode_icon = match app.play_mode {
        PlayMode::RepeatOne => "\u{21BA}1",
        PlayMode::Sequential => "\u{21BA}",
        PlayMode::Shuffle => "\u{21C4}",
    };
    let mode_block = Paragraph::new(Line::from(Span::styled(
        mode_icon,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )))
    .alignment(ratatui::layout::Alignment::Center)
    .block(Block::default().borders(Borders::ALL).title(" Mode "));
    frame.render_widget(mode_block, status_chunks[2]);

    let help = Paragraph::new(Line::from(vec![
        Span::styled("[a]", Style::default().fg(Color::Green)),
        Span::raw(" Add  "),
        Span::styled("[Enter]", Style::default().fg(Color::Green)),
        Span::raw(" Play  "),
        Span::styled("[Space]", Style::default().fg(Color::Green)),
        Span::raw(" Pause  "),
        Span::styled("[r]", Style::default().fg(Color::Green)),
        Span::raw(" Reset  "),
        Span::styled("[d]", Style::default().fg(Color::Green)),
        Span::raw(" Delete  "),
        // Span::styled("[q]", Style::default().fg(Color::Green)),
        // Span::raw(" Quit  "),
        Span::styled("[m]", Style::default().fg(Color::Green)),
        Span::raw(" Mode  "),
        Span::styled("[1-n]", Style::default().fg(Color::Green)),
        Span::raw(" Goto"),
    ]));
    frame.render_widget(help, chunks[2]);
}

fn draw_add_song(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let vertical = Layout::vertical([Constraint::Percentage(30); 3]);
    let input_area = vertical.split(area)[1];

    let horizontal = Layout::horizontal([
        Constraint::Percentage(20),
        Constraint::Percentage(60),
        Constraint::Percentage(20),
    ]);
    let input_area = horizontal.split(input_area)[1];

    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
    ])
    .split(input_area);

    let url_style = if app.input_focus == InputField::Url {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let url_input = Paragraph::new(app.input_url.as_str())
        .style(url_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Full URL (the url page with music) "),
        );
    frame.render_widget(url_input, chunks[0]);

    let alias_style = if app.input_focus == InputField::Alias {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let alias_input = Paragraph::new(app.input_alias.as_str())
        .style(alias_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Alias (friendly name) "),
        );
    frame.render_widget(alias_input, chunks[1]);

    let (cx, cy) = match app.input_focus {
        InputField::Url => {
            let r = chunks[0];
            (r.x + 1 + app.input_url.len() as u16, r.y + 1)
        }
        InputField::Alias => {
            let r = chunks[1];
            (r.x + 1 + app.input_alias.len() as u16, r.y + 1)
        }
    };
    frame.set_cursor_position((cx, cy));

    let help = Paragraph::new(Line::from(vec![
        Span::styled("[Tab]", Style::default().fg(Color::Green)),
        Span::raw(" Switch  "),
        Span::styled("[Enter]", Style::default().fg(Color::Green)),
        Span::raw(" Confirm  "),
        Span::styled("[Esc]", Style::default().fg(Color::Green)),
        Span::raw(" Cancel"),
    ]));
    frame.render_widget(help, chunks[2]);
}

fn draw_edit_alias(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let vertical = Layout::vertical([Constraint::Percentage(35); 3]);
    let input_area = vertical.split(area)[1];

    let horizontal = Layout::horizontal([
        Constraint::Percentage(20),
        Constraint::Percentage(60),
        Constraint::Percentage(20),
    ]);
    let input_area = horizontal.split(input_area)[1];

    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Length(3)]).split(input_area);

    let alias_input = Paragraph::new(app.input_alias.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title(" Edit alias "));
    frame.render_widget(alias_input, chunks[0]);

    let r = chunks[0];
    frame.set_cursor_position((r.x + 1 + app.input_alias.len() as u16, r.y + 1));

    let help = Paragraph::new(Line::from(vec![
        Span::styled("[Enter]", Style::default().fg(Color::Green)),
        Span::raw(" Save  "),
        Span::styled("[Esc]", Style::default().fg(Color::Green)),
        Span::raw(" Cancel"),
    ]));
    frame.render_widget(help, chunks[1]);
}

fn draw_confirm_delete(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let vert = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(7),
        Constraint::Min(0),
    ])
    .split(area);

    let horiz = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(38),
        Constraint::Min(0),
    ])
    .split(vert[1]);

    frame.render_widget(Clear, horiz[1]);

    let alias = app
        .playlist
        .songs
        .get(app.delete_target)
        .map(|s| s.alias.as_str())
        .unwrap_or("unknown");

    let text = vec![
        Line::from(format!(" Delete \"{}\"?", alias)),
        Line::from(String::new()),
        Line::from(vec![
            Span::styled("[y]", Style::default().fg(Color::Green)),
            Span::raw(" Yes  "),
            Span::styled("[n]", Style::default().fg(Color::Green)),
            Span::raw(" No"),
        ]),
    ];

    let dialog = Paragraph::new(text)
        .alignment(ratatui::layout::Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Yellow))
                .title(" Confirm "),
        );

    frame.render_widget(dialog, horiz[1]);
}

fn draw_error_dialog(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let vert = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(8),
        Constraint::Min(0),
    ])
    .split(area);

    let horiz = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(54),
        Constraint::Min(0),
    ])
    .split(vert[1]);

    frame.render_widget(Clear, horiz[1]);

    let text = vec![
        Line::from(String::new()),
        Line::from(app.error_message.as_str()),
        Line::from(String::new()),
        Line::from(vec![
            Span::styled("[Enter]", Style::default().fg(Color::Green)),
            Span::raw(" OK"),
        ]),
    ];

    let dialog = Paragraph::new(text)
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(ratatui::widgets::Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Red))
                .title(" Error "),
        );

    frame.render_widget(dialog, horiz[1]);
}
