use anyhow::Result;
use rand::RngExt;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PlayMode {
    RepeatOne,
    #[default]
    Sequential,
    Shuffle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Song {
    pub url: String,
    pub alias: String,
    #[serde(default)]
    pub last_position: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Playlist {
    pub songs: Vec<Song>,
    #[serde(default)]
    pub play_mode: PlayMode,
    #[serde(default)]
    pub last_played_alias: Option<String>,
}

/// Returns the next song index given the current play mode and state.
/// The caller must ensure `total_songs > 0`.
pub fn next_song_index(play_mode: PlayMode, current_index: usize, total_songs: usize) -> usize {
    match play_mode {
        PlayMode::RepeatOne => current_index,
        PlayMode::Sequential => (current_index + 1) % total_songs,
        PlayMode::Shuffle => {
            let mut rng = rand::rng();
            let mut idx = rng.random_range(0..total_songs);
            if total_songs > 1 && idx == current_index {
                idx = (idx + 1) % total_songs;
            }
            idx
        }
    }
}

impl Playlist {
    pub fn load() -> Result<Self> {
        let path = Self::path();
        if path.exists() {
            let data = std::fs::read_to_string(&path)?;
            Ok(serde_json::from_str(&data)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, data)?;
        Ok(())
    }

    fn path() -> PathBuf {
        let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("terminal-music-player").join("playlist.json")
    }
}
