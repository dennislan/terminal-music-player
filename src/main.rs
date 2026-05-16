use anyhow::Result;
use crossterm::ExecutableCommand;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;
use std::time::Duration;
use terminal_music_player::app::{App, InputField, Screen};
use terminal_music_player::ui;

fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new()?;

    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()>
where
    <B as ratatui::backend::Backend>::Error: Send + Sync + 'static,
{
    loop {
        app.check_status_timer();
        app.poll_events();
        terminal.draw(|f| ui::draw(f, app))?;

        if app.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.screen {
                        Screen::Playlist => handle_playlist_key(app, key),
                        Screen::AddSong => handle_add_song_key(app, key),
                        Screen::EditAlias => handle_edit_alias_key(app, key),
                        Screen::ConfirmDelete => handle_confirm_delete_key(app, key),
                        Screen::Error => handle_error_key(app, key),
                    }
                }
            }
        }
    }

    Ok(())
}

fn handle_playlist_key(app: &mut App, key: event::KeyEvent) {
    match key.code {
        KeyCode::Char('q') => {
            app.save_last_played();
            app.save_current_position_quiet();
            app.save_playlist();
            app.should_quit = true;
        }
        KeyCode::Char(c) if c.is_ascii_digit() && app.number_input.is_none() => {
            app.start_number_input(c);
        }
        KeyCode::Char(c) if c.is_ascii_digit() && app.number_input.is_some() => {
            app.append_number_input(c);
        }
        KeyCode::Char('a') => {
            app.screen = Screen::AddSong;
            app.input_url.clear();
            app.input_alias.clear();
            app.input_focus = InputField::Url;
            app.status_message = None;
        }
        KeyCode::Char('d') => {
            if !app.playlist.songs.is_empty() {
                app.delete_target = app.selected_index;
                app.screen = Screen::ConfirmDelete;
                app.status_message = None;
            }
        }
        KeyCode::Char('e') => {
            app.edit_selected();
        }
        KeyCode::Enter => {
            if app.number_input.is_some() {
                app.confirm_number_input();
            } else {
                app.play_selected();
            }
        }
        KeyCode::Esc => {
            if app.number_input.is_some() {
                app.cancel_number_input();
            }
        }
        KeyCode::Char(' ') => {
            app.toggle_pause();
        }
        KeyCode::Char('r') => {
            app.reset_position();
        }
        KeyCode::Up if key.modifiers.contains(KeyModifiers::ALT) => {
            app.move_song_up();
        }
        KeyCode::Down if key.modifiers.contains(KeyModifiers::ALT) => {
            app.move_song_down();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.select_up();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.select_down();
        }
        KeyCode::Left => {
            app.seek_relative(-10.0);
        }
        KeyCode::Right => {
            app.seek_relative(10.0);
        }
        KeyCode::Char('m') => {
            app.cycle_play_mode();
        }
        _ => {}
    }
}

fn handle_edit_alias_key(app: &mut App, key: event::KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.screen = Screen::Playlist;
            app.status_message = None;
        }
        KeyCode::Enter => {
            app.update_alias();
        }
        KeyCode::Char(c) => {
            app.input_alias.push(c);
        }
        KeyCode::Backspace => {
            app.input_alias.pop();
        }
        _ => {}
    }
}

fn handle_confirm_delete_key(app: &mut App, key: event::KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Enter => {
            let target = app.delete_target;
            app.selected_index = target;
            app.delete_selected();
            app.screen = Screen::Playlist;
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            app.screen = Screen::Playlist;
            app.status_message = None;
        }
        _ => {}
    }
}

fn handle_error_key(app: &mut App, _key: event::KeyEvent) {
    app.screen = Screen::Playlist;
}

fn handle_add_song_key(app: &mut App, key: event::KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.screen = Screen::Playlist;
            app.status_message = None;
        }
        KeyCode::Tab => {
            app.input_focus = match app.input_focus {
                InputField::Url => InputField::Alias,
                InputField::Alias => InputField::Url,
            };
        }
        KeyCode::Enter => {
            app.add_song();
        }
        KeyCode::Char(c) => match app.input_focus {
            InputField::Url => app.input_url.push(c),
            InputField::Alias => app.input_alias.push(c),
        },
        KeyCode::Backspace => match app.input_focus {
            InputField::Url => {
                app.input_url.pop();
            }
            InputField::Alias => {
                app.input_alias.pop();
            }
        },
        _ => {}
    }
}
