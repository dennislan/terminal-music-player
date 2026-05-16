use crate::player::{Player, PlayerCommand, PlayerEvent};
pub use crate::playlist::PlayMode;
use crate::playlist::{Playlist, Song, next_song_index};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Playlist,
    AddSong,
    EditAlias,
    ConfirmDelete,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputField {
    Url,
    Alias,
}

pub struct App {
    pub playlist: Playlist,
    pub player: Player,
    pub screen: Screen,
    pub selected_index: usize,
    pub current_song: Option<String>,
    pub is_paused: bool,
    pub is_buffering: bool,
    pub current_position: f64,
    pub total_duration: f64,
    pub status_message: Option<String>,
    pub input_url: String,
    pub input_alias: String,
    pub input_focus: InputField,
    pub should_quit: bool,
    pub play_mode: PlayMode,
    pub status_restore: Option<(String, Instant)>,
    pub delete_target: usize,
    pub error_message: String,
    /// Number input buffer for "go to song by number" feature.
    /// When Some, the user is typing a song number.
    pub number_input: Option<String>,
    /// Pending play index when the user quickly requests multiple songs.
    /// Only the last request is kept; previous ones are cancelled.
    pub pending_play_index: Option<usize>,
}

impl App {
    pub fn new() -> anyhow::Result<Self> {
        let playlist = Playlist::load()?;
        let player = Player::new();

        let selected_index = playlist
            .last_played_alias
            .as_ref()
            .and_then(|alias| playlist.songs.iter().position(|s| &s.alias == alias))
            .unwrap_or(0);

        let status_message = if playlist.songs.is_empty() {
            Some("Press [a] to add a URL to play".to_string())
        } else {
            Some("Press [Enter] to play".to_string())
        };
        let play_mode = playlist.play_mode;

        Ok(App {
            playlist,
            player,
            screen: Screen::Playlist,
            selected_index,
            current_song: None,
            is_paused: false,
            is_buffering: false,
            current_position: 0.0,
            total_duration: 0.0,
            status_message,
            input_url: String::new(),
            input_alias: String::new(),
            input_focus: InputField::Url,
            should_quit: false,
            play_mode,
            status_restore: None,
            delete_target: 0,
            error_message: String::new(),
            number_input: None,
            pending_play_index: None,
        })
    }

    fn save_current_position(&mut self) {
        if let Some(ref alias) = self.current_song {
            let pos = self.current_position;
            if let Some(song) = self.playlist.songs.iter_mut().find(|s| &s.alias == alias) {
                song.last_position = pos;
            }
        }
    }

    pub fn poll_events(&mut self) {
        while let Some(event) = self.player.try_recv() {
            match event {
                PlayerEvent::Buffering(alias) => self.on_buffering(alias),
                PlayerEvent::Started(alias, duration) => self.on_started(alias, duration),
                PlayerEvent::Position(pos) => self.on_position(pos),
                PlayerEvent::Paused => self.on_paused(),
                PlayerEvent::Resumed => self.on_resumed(),
                PlayerEvent::Finished => self.on_finished(),
                PlayerEvent::Stopped => self.on_stopped(),
                PlayerEvent::Error(msg) => self.on_player_error(msg),
            }
        }
    }

    fn on_stopped(&mut self) {
        if let Some(idx) = self.pending_play_index.take() {
            self.selected_index = idx;
            self.play_selected();
        }
    }

    fn on_buffering(&mut self, alias: String) {
        self.is_buffering = true;
        self.current_song = Some(alias.clone());
        self.status_message = Some(format!("Downloading: {}", alias));
    }

    fn on_started(&mut self, alias: String, duration: f64) {
        self.is_buffering = false;
        self.current_song = Some(alias.clone());
        self.is_paused = false;
        if duration > 0.0 {
            self.total_duration = duration;
        }
        self.status_message = Some(format!("Now playing: {}", alias));
    }

    fn on_position(&mut self, pos: std::time::Duration) {
        self.current_position = pos.as_secs_f64();
    }

    fn on_paused(&mut self) {
        self.is_paused = true;
        self.save_current_position();
        self.status_message = Some("Paused".to_string());
    }

    fn on_resumed(&mut self) {
        self.is_paused = false;
        self.status_message = Some("Resumed".to_string());
    }

    fn on_finished(&mut self) {
        // Reset last_position to 0 so replay (RepeatOne) doesn't seek to end
        if let Some(ref alias) = self.current_song {
            if let Some(song) = self.playlist.songs.iter_mut().find(|s| s.alias == *alias) {
                song.last_position = 0.0;
            }
        }
        self.save_playlist();
        self.current_song = None;
        self.is_paused = false;
        self.is_buffering = false;
        self.current_position = 0.0;

        if self.playlist.songs.is_empty() {
            self.status_message = Some("No songs. Press [a] to add a URL.".to_string());
            return;
        }

        let next = next_song_index(
            self.play_mode,
            self.selected_index,
            self.playlist.songs.len(),
        );
        self.selected_index = next;
        self.play_selected();
        self.status_message = Some(format!(
            "Auto-playing next: {}",
            self.playlist.songs[next].alias
        ));
    }

    fn on_player_error(&mut self, msg: String) {
        self.is_buffering = false;
        if msg.starts_with("Stream error:") || msg.starts_with("Audio download error:") {
            self.screen = Screen::Error;
            self.error_message =
                "Cannot play. Please confirm the URL has an music stream."
                    .to_string();
            self.status_message = None;
            self.current_song = None;
            self.is_paused = false;
            self.current_position = 0.0;
            self.total_duration = 0.0;
        } else {
            self.status_message = Some(format!("Error: {}", msg));
        }
    }

    pub fn save_last_played(&mut self) {
        let alias = self.current_song.clone().or_else(|| {
            self.playlist
                .songs
                .get(self.selected_index)
                .map(|s| s.alias.clone())
        });
        self.playlist.last_played_alias = alias;
    }

    pub fn save_playlist(&self) {
        if let Err(e) = self.playlist.save() {
            eprintln!("Failed to save playlist: {}", e);
        }
    }

    pub fn save_current_position_quiet(&mut self) {
        if let Some(ref alias) = self.current_song.clone() {
            let pos = self.current_position;
            if let Some(song) = self.playlist.songs.iter_mut().find(|s| &s.alias == alias) {
                song.last_position = pos;
            }
        }
    }

    pub fn reset_position(&mut self) {
        let current = self.current_song.clone();
        if let Some(ref alias) = current {
            if let Some(idx) = self.playlist.songs.iter().position(|s| &s.alias == alias) {
                self.selected_index = idx;
            }
            if let Some(song) = self.playlist.songs.iter_mut().find(|s| &s.alias == alias) {
                song.last_position = 0.0;
            }
            self.save_playlist();
            self.current_position = 0.0;
            self.is_paused = false;
            self.play_selected();
            self.status_message = Some("Restarting from beginning".to_string());
        }
    }

    pub fn play_selected(&mut self) {
        self.pending_play_index = None;
        self.save_current_position();
        self.save_playlist();
        if let Some(song) = self.playlist.songs.get(self.selected_index) {
            self.is_buffering = true;
            self.current_song = Some(song.alias.clone());
            self.current_position = 0.0;
            self.total_duration = 0.0;
            self.status_message = Some(format!("Downloading: {} ...", song.alias));

            self.player.send(PlayerCommand::Play(song.clone()));
        }
    }

    pub fn toggle_pause(&self) {
        self.player.send(PlayerCommand::PauseResume);
    }

    pub fn add_song(&mut self) {
        let url = self.input_url.trim().to_string();
        let alias = self.input_alias.trim().to_string();

        if url.is_empty() || alias.is_empty() {
            self.status_message = Some("Both URL and alias are required".to_string());
            return;
        }

        if self.playlist.songs.iter().any(|s| s.alias == alias) {
            self.status_message = Some("Alias already exists".to_string());
            return;
        }

        self.playlist.songs.push(Song {
            url,
            alias,
            last_position: 0.0,
        });

        if let Err(e) = self.playlist.save() {
            self.status_message = Some(format!("Save error: {}", e));
        } else {
            self.status_message = Some("Song added".to_string());
        }

        self.input_url.clear();
        self.input_alias.clear();
        self.screen = Screen::Playlist;
    }

    pub fn delete_selected(&mut self) {
        if self.selected_index >= self.playlist.songs.len() {
            return;
        }

        if let Some(ref alias) = self.current_song {
            if let Some(song) = self.playlist.songs.get(self.selected_index) {
                if song.alias == *alias {
                    self.player.send(PlayerCommand::Stop);
                    self.current_song = None;
                    self.is_paused = false;
                    self.is_buffering = false;
                    self.current_position = 0.0;
                    self.total_duration = 0.0;
                }
            }
        }

        self.playlist.songs.remove(self.selected_index);
        if self.selected_index > 0 && self.selected_index >= self.playlist.songs.len() {
            self.selected_index = self.playlist.songs.len().saturating_sub(1);
        }
        if let Err(e) = self.playlist.save() {
            self.status_message = Some(format!("Save error: {}", e));
        }
    }

    pub fn edit_selected(&mut self) {
        if let Some(song) = self.playlist.songs.get(self.selected_index) {
            self.input_url = song.url.clone();
            self.input_alias = song.alias.clone();
            self.input_focus = InputField::Alias;
            self.screen = Screen::EditAlias;
            self.status_message = None;
        }
    }

    pub fn move_song_up(&mut self) {
        if self.playlist.songs.len() < 2 || self.selected_index == 0 {
            return;
        }
        let i = self.selected_index;
        self.playlist.songs.swap(i, i - 1);
        self.selected_index -= 1;
        if let Err(e) = self.playlist.save() {
            self.status_message = Some(format!("Save error: {}", e));
        }
    }

    pub fn move_song_down(&mut self) {
        if self.playlist.songs.len() < 2 || self.selected_index >= self.playlist.songs.len() - 1 {
            return;
        }
        let i = self.selected_index;
        self.playlist.songs.swap(i, i + 1);
        self.selected_index += 1;
        if let Err(e) = self.playlist.save() {
            self.status_message = Some(format!("Save error: {}", e));
        }
    }

    pub fn update_alias(&mut self) {
        let new_alias = self.input_alias.trim().to_string();
        if new_alias.is_empty() {
            self.status_message = Some("Alias cannot be empty".to_string());
            return;
        }
        if let Some(song) = self.playlist.songs.get_mut(self.selected_index) {
            song.alias = new_alias;
        }
        if let Err(e) = self.playlist.save() {
            self.status_message = Some(format!("Save error: {}", e));
        } else {
            self.status_message = Some("Alias updated".to_string());
        }
        self.screen = Screen::Playlist;
    }

    pub fn cycle_play_mode(&mut self) {
        self.play_mode = match self.play_mode {
            PlayMode::RepeatOne => PlayMode::Sequential,
            PlayMode::Sequential => PlayMode::Shuffle,
            PlayMode::Shuffle => PlayMode::RepeatOne,
        };
        self.playlist.play_mode = self.play_mode;
        self.save_playlist();
        let name = match self.play_mode {
            PlayMode::RepeatOne => "Repeat One",
            PlayMode::Sequential => "Sequential",
            PlayMode::Shuffle => "Shuffle",
        };
        let prev = self.status_message.clone().unwrap_or_default();
        self.status_restore = Some((prev, Instant::now()));
        self.status_message = Some(format!("Mode: {}", name));
    }

    pub fn check_status_timer(&mut self) {
        if let Some((ref prev, start)) = self.status_restore {
            if start.elapsed() >= std::time::Duration::from_secs(1) {
                self.status_message = if prev.is_empty() {
                    None
                } else {
                    Some(prev.clone())
                };
                self.status_restore = None;
            }
        }
    }

    pub fn seek_relative(&mut self, offset_secs: f64) {
        if self.current_song.is_none() {
            return;
        }
        let new_pos = (self.current_position + offset_secs).clamp(0.0, self.total_duration);
        self.player.send(PlayerCommand::SeekTo(new_pos));
        self.current_position = new_pos;
        if offset_secs < 0.0 {
            self.status_message = Some(format!("Seek: {}s back", offset_secs.abs() as i64));
        } else {
            self.status_message = Some(format!("Seek: {}s forward", offset_secs as i64));
        }
    }

    pub fn select_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn select_down(&mut self) {
        if self.selected_index + 1 < self.playlist.songs.len() {
            self.selected_index += 1;
        }
    }

    /// Start number input mode with the first digit.
    /// Song numbers are 1-based (1 = first song).
    pub fn start_number_input(&mut self, digit: char) {
        self.number_input = Some(digit.to_string());
        self.status_message = Some(format!("Go to: {}", self.number_input.as_ref().unwrap()));
    }

    /// Append a digit to the current number input.
    pub fn append_number_input(&mut self, digit: char) {
        if let Some(ref mut buf) = self.number_input {
            buf.push(digit);
            self.status_message = Some(format!("Go to: {}", buf));
        }
    }

    /// Confirm number input: parse the number and play the corresponding song.
    /// Song numbers are 1-based (1 = first song).
    /// If a song is currently playing/buffering, stop it and wait for Stopped
    /// before playing the new song. Rapid inputs only keep the last request.
    pub fn confirm_number_input(&mut self) {
        let buf = self.number_input.take();
        if let Some(ref num_str) = buf {
            if let Ok(num) = num_str.parse::<usize>() {
                if num >= 1 && num <= self.playlist.songs.len() {
                    let idx = num - 1;
                    self.pending_play_index = Some(idx);
                    // Stop current playback so we can play the pending song.
                    // The Stopped event will trigger the actual play.
                    if self.current_song.is_some() || self.is_buffering {
                        self.player.send(PlayerCommand::Stop);
                    } else {
                        // Player is idle, play immediately.
                        self.selected_index = idx;
                        self.pending_play_index = None;
                        self.play_selected();
                    }
                    return;
                }
            }
        }
        self.pending_play_index = None;
        self.status_message = Some("Invalid song number".to_string());
    }

    /// Cancel number input mode and restore status.
    pub fn cancel_number_input(&mut self) {
        self.number_input = None;
        self.pending_play_index = None;
        self.status_message = if self.playlist.songs.is_empty() {
            Some("Press [a] to add a URL to play".to_string())
        } else if let Some(ref alias) = self.current_song {
            if self.is_paused {
                Some(format!("Paused: {}", alias))
            } else if self.is_buffering {
                Some(format!("Downloading: {} ...", alias))
            } else {
                Some(format!("Now playing: {}", alias))
            }
        } else {
            Some("Press [Enter] to play".to_string())
        };
    }
}
