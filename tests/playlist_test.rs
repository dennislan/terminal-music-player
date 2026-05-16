use std::fs;
use tempfile::TempDir;
use terminal_music_player::playlist::{PlayMode, Playlist, Song, next_song_index};

#[test]
fn test_sequential_next() {
    assert_eq!(next_song_index(PlayMode::Sequential, 0, 5), 1);
    assert_eq!(next_song_index(PlayMode::Sequential, 4, 5), 0);
}

#[test]
fn test_repeat_one() {
    assert_eq!(next_song_index(PlayMode::RepeatOne, 2, 10), 2);
    assert_eq!(next_song_index(PlayMode::RepeatOne, 0, 1), 0);
}

#[test]
fn test_shuffle_never_same_when_multi() {
    for _ in 0..20 {
        let n = next_song_index(PlayMode::Shuffle, 3, 5);
        assert_ne!(n, 3, "shuffle should not repeat current when total > 1");
        assert!(n < 5);
    }
}

#[test]
fn test_shuffle_single_song() {
    assert_eq!(next_song_index(PlayMode::Shuffle, 0, 1), 0);
}

#[test]
fn test_playlist_save_and_load() {
    let temp_dir = TempDir::new().unwrap();
    let playlist = Playlist {
        songs: vec![
            Song {
                url: "https://example.com/1".to_string(),
                alias: "Song 1".to_string(),
                last_position: 10.5,
            },
            Song {
                url: "https://example.com/2".to_string(),
                alias: "Song 2".to_string(),
                last_position: 0.0,
            },
        ],
        play_mode: PlayMode::RepeatOne,
        last_played_alias: Some("Song 1".to_string()),
    };

    // Test save
    let path = temp_dir.path().join("test_playlist.json");
    let json = serde_json::to_string_pretty(&playlist).unwrap();
    fs::write(&path, &json).unwrap();

    // Test load
    let loaded: Playlist = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(loaded.songs.len(), 2);
    assert_eq!(loaded.songs[0].alias, "Song 1");
    assert_eq!(loaded.songs[0].last_position, 10.5);
    assert_eq!(loaded.play_mode, PlayMode::RepeatOne);
    assert_eq!(loaded.last_played_alias, Some("Song 1".to_string()));
}

#[test]
fn test_playlist_default() {
    let playlist = Playlist::default();
    assert_eq!(playlist.songs.len(), 0);
    assert_eq!(playlist.play_mode, PlayMode::Sequential);
    assert_eq!(playlist.last_played_alias, None);
}

#[test]
fn test_song_serialization() {
    let song = Song {
        url: "https://example.com".to_string(),
        alias: "Test".to_string(),
        last_position: 42.0,
    };

    let json = serde_json::to_string(&song).unwrap();
    let deserialized: Song = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.url, song.url);
    assert_eq!(deserialized.alias, song.alias);
    assert_eq!(deserialized.last_position, song.last_position);
}

#[test]
fn test_play_mode_default() {
    assert_eq!(PlayMode::default(), PlayMode::Sequential);
}
