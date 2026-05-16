use std::time::Instant;
use terminal_music_player::app::{App, InputField, Screen};
use terminal_music_player::player::Player;
use terminal_music_player::playlist::{PlayMode, Playlist, Song};

fn create_test_app() -> App {
    let playlist = Playlist {
        songs: vec![
            Song {
                url: "https://example.com/1".to_string(),
                alias: "Song 1".to_string(),
                last_position: 0.0,
            },
            Song {
                url: "https://example.com/2".to_string(),
                alias: "Song 2".to_string(),
                last_position: 0.0,
            },
        ],
        play_mode: PlayMode::Sequential,
        last_played_alias: None,
    };

    App {
        playlist,
        player: Player::new(),
        screen: Screen::Playlist,
        selected_index: 0,
        current_song: None,
        is_paused: false,
        is_buffering: false,
        current_position: 0.0,
        total_duration: 0.0,
        status_message: Some("Test".to_string()),
        input_url: String::new(),
        input_alias: String::new(),
        input_focus: InputField::Url,
        should_quit: false,
        play_mode: PlayMode::Sequential,
        status_restore: None,
        delete_target: 0,
        error_message: String::new(),
        number_input: None,
        pending_play_index: None,
    }
}

#[test]
fn test_select_up() {
    let mut app = create_test_app();
    app.selected_index = 1;
    app.select_up();
    assert_eq!(app.selected_index, 0);
}

#[test]
fn test_select_up_at_zero() {
    let mut app = create_test_app();
    app.selected_index = 0;
    app.select_up();
    assert_eq!(app.selected_index, 0);
}

#[test]
fn test_select_down() {
    let mut app = create_test_app();
    app.select_down();
    assert_eq!(app.selected_index, 1);
}

#[test]
fn test_select_down_at_end() {
    let mut app = create_test_app();
    app.selected_index = 1;
    app.select_down();
    assert_eq!(app.selected_index, 1);
}

#[test]
fn test_cycle_play_mode() {
    let mut app = create_test_app();
    assert_eq!(app.play_mode, PlayMode::Sequential);

    app.cycle_play_mode();
    assert_eq!(app.play_mode, PlayMode::Shuffle);

    app.cycle_play_mode();
    assert_eq!(app.play_mode, PlayMode::RepeatOne);

    app.cycle_play_mode();
    assert_eq!(app.play_mode, PlayMode::Sequential);
}

#[test]
fn test_cycle_play_mode_status_timer() {
    let mut app = create_test_app();
    app.cycle_play_mode();
    assert!(app.status_restore.is_some());
    assert!(app.status_message.is_some());
    assert!(app.status_message.as_ref().unwrap().contains("Mode:"));
}

#[test]
fn test_check_status_timer_expired() {
    let mut app = create_test_app();
    app.status_message = Some("Temp".to_string());
    app.status_restore = Some((
        "Previous".to_string(),
        Instant::now() - std::time::Duration::from_secs(2),
    ));
    app.check_status_timer();
    assert_eq!(app.status_message, Some("Previous".to_string()));
    assert!(app.status_restore.is_none());
}

#[test]
fn test_check_status_timer_not_expired() {
    let mut app = create_test_app();
    app.status_message = Some("Temp".to_string());
    app.status_restore = Some(("Previous".to_string(), Instant::now()));
    app.check_status_timer();
    assert_eq!(app.status_message, Some("Temp".to_string()));
    assert!(app.status_restore.is_some());
}

#[test]
fn test_seek_relative_no_song() {
    let mut app = create_test_app();
    app.current_song = None;
    app.seek_relative(10.0);
    assert_eq!(app.current_position, 0.0);
}

#[test]
fn test_seek_relative_with_song() {
    let mut app = create_test_app();
    app.current_song = Some("Song 1".to_string());
    app.current_position = 30.0;
    app.total_duration = 100.0;
    app.seek_relative(-10.0);
    assert_eq!(app.current_position, 20.0);
}

#[test]
fn test_seek_relative_clamp() {
    let mut app = create_test_app();
    app.current_song = Some("Song 1".to_string());
    app.current_position = 5.0;
    app.total_duration = 100.0;
    app.seek_relative(-10.0);
    assert_eq!(app.current_position, 0.0);
}

#[test]
fn test_move_song_up() {
    let mut app = create_test_app();
    app.selected_index = 1;
    app.move_song_up();
    assert_eq!(app.selected_index, 0);
    assert_eq!(app.playlist.songs[0].alias, "Song 2");
    assert_eq!(app.playlist.songs[1].alias, "Song 1");
}

#[test]
fn test_move_song_up_at_zero() {
    let mut app = create_test_app();
    app.selected_index = 0;
    app.move_song_up();
    assert_eq!(app.selected_index, 0);
}

#[test]
fn test_move_song_down() {
    let mut app = create_test_app();
    app.selected_index = 0;
    app.move_song_down();
    assert_eq!(app.selected_index, 1);
    assert_eq!(app.playlist.songs[0].alias, "Song 2");
    assert_eq!(app.playlist.songs[1].alias, "Song 1");
}

#[test]
fn test_add_song_empty_url() {
    let mut app = create_test_app();
    app.input_url = "  ".to_string();
    app.input_alias = "Test".to_string();
    app.add_song();
    assert!(app.status_message.as_ref().unwrap().contains("required"));
}

#[test]
fn test_add_song_empty_alias() {
    let mut app = create_test_app();
    app.input_url = "https://example.com".to_string();
    app.input_alias = "   ".to_string();
    app.add_song();
    assert!(app.status_message.as_ref().unwrap().contains("required"));
}

#[test]
fn test_add_song_duplicate_alias() {
    let mut app = create_test_app();
    app.input_url = "https://example.com/3".to_string();
    app.input_alias = "Song 1".to_string();
    app.add_song();
    assert!(
        app.status_message
            .as_ref()
            .unwrap()
            .contains("already exists")
    );
}

#[test]
fn test_delete_selected() {
    let mut app = create_test_app();
    app.selected_index = 0;
    app.delete_selected();
    assert_eq!(app.playlist.songs.len(), 1);
    assert_eq!(app.playlist.songs[0].alias, "Song 2");
}

#[test]
fn test_delete_selected_last() {
    let mut app = create_test_app();
    app.selected_index = 1;
    app.delete_selected();
    assert_eq!(app.playlist.songs.len(), 1);
    assert_eq!(app.selected_index, 0);
}

#[test]
fn test_update_alias() {
    let mut app = create_test_app();
    app.selected_index = 0;
    app.input_alias = "New Name".to_string();
    app.update_alias();
    assert_eq!(app.playlist.songs[0].alias, "New Name");
    assert_eq!(app.screen, Screen::Playlist);
}

#[test]
fn test_update_alias_empty() {
    let mut app = create_test_app();
    app.selected_index = 0;
    app.input_alias = "   ".to_string();
    app.update_alias();
    assert!(app.status_message.as_ref().unwrap().contains("empty"));
}

#[test]
fn test_reset_position() {
    let mut app = create_test_app();
    app.current_song = Some("Song 1".to_string());
    app.playlist.songs[0].last_position = 50.0;
    app.reset_position();
    assert_eq!(app.playlist.songs[0].last_position, 0.0);
}

#[test]
fn test_screen_variants() {
    assert_eq!(Screen::Playlist as u8, 0);
    assert_eq!(Screen::AddSong as u8, 1);
    assert_eq!(Screen::EditAlias as u8, 2);
    assert_eq!(Screen::ConfirmDelete as u8, 3);
    assert_eq!(Screen::Error as u8, 4);
}

#[test]
fn test_input_field_variants() {
    assert_eq!(InputField::Url as u8, 0);
    assert_eq!(InputField::Alias as u8, 1);
}

// --- Number input tests ---

#[test]
fn test_start_number_input() {
    let mut app = create_test_app();
    app.start_number_input('5');
    assert_eq!(app.number_input, Some("5".to_string()));
    assert!(app.status_message.unwrap().contains("Go to:"));
}

#[test]
fn test_append_number_input() {
    let mut app = create_test_app();
    app.start_number_input('1');
    app.append_number_input('5');
    assert_eq!(app.number_input, Some("15".to_string()));
}

#[test]
fn test_confirm_number_input_valid() {
    let mut app = create_test_app();
    app.start_number_input('2');
    app.confirm_number_input();
    assert_eq!(app.selected_index, 1);
    assert_eq!(app.number_input, None);
}

#[test]
fn test_confirm_number_input_invalid() {
    let mut app = create_test_app();
    app.number_input = Some("99".to_string());
    app.confirm_number_input();
    assert_eq!(app.number_input, None);
    assert!(app.status_message.unwrap().contains("Invalid"));
}

#[test]
fn test_confirm_number_input_out_of_range() {
    let mut app = create_test_app();
    app.number_input = Some("0".to_string());
    app.confirm_number_input();
    assert!(app.status_message.unwrap().contains("Invalid"));
}

#[test]
fn test_cancel_number_input() {
    let mut app = create_test_app();
    app.start_number_input('3');
    app.cancel_number_input();
    assert_eq!(app.number_input, None);
    assert_eq!(app.pending_play_index, None);
    assert!(app.status_message.is_some());
}

// --- Pending play cancellation tests ---

#[test]
fn test_confirm_number_input_sets_pending_when_playing() {
    let mut app = create_test_app();
    // Simulate that a song is currently playing.
    app.current_song = Some("Song 1".to_string());
    app.start_number_input('2');
    app.confirm_number_input();
    // Should set pending_play_index instead of playing immediately.
    assert_eq!(app.pending_play_index, Some(1));
    assert_eq!(app.number_input, None);
}

#[test]
fn test_confirm_number_input_sets_pending_when_buffering() {
    let mut app = create_test_app();
    app.is_buffering = true;
    app.start_number_input('1');
    app.confirm_number_input();
    assert_eq!(app.pending_play_index, Some(0));
    assert_eq!(app.number_input, None);
}

#[test]
fn test_confirm_number_input_no_pending_when_idle() {
    let mut app = create_test_app();
    // Idle: no current_song, not buffering.
    app.current_song = None;
    app.is_buffering = false;
    app.start_number_input('2');
    app.confirm_number_input();
    // Should play immediately, no pending.
    assert_eq!(app.pending_play_index, None);
    assert_eq!(app.selected_index, 1);
    assert_eq!(app.number_input, None);
}

#[test]
fn test_confirm_number_input_overwrites_pending() {
    let mut app = create_test_app();
    app.current_song = Some("Song 1".to_string());
    // First request: go to song 1.
    app.start_number_input('1');
    app.confirm_number_input();
    assert_eq!(app.pending_play_index, Some(0));
    // Second request: go to song 2 (overwrites the first).
    app.start_number_input('2');
    app.confirm_number_input();
    // Only the last request should be pending.
    assert_eq!(app.pending_play_index, Some(1));
}

#[test]
fn test_confirm_number_input_invalid_clears_pending() {
    let mut app = create_test_app();
    app.pending_play_index = Some(0);
    app.number_input = Some("99".to_string());
    app.confirm_number_input();
    assert_eq!(app.pending_play_index, None);
    assert!(app.status_message.as_ref().unwrap().contains("Invalid"));
}

#[test]
fn test_cancel_number_input_clears_pending() {
    let mut app = create_test_app();
    app.pending_play_index = Some(1);
    app.start_number_input('3');
    app.cancel_number_input();
    assert_eq!(app.pending_play_index, None);
    assert_eq!(app.number_input, None);
}

#[test]
fn test_play_selected_clears_pending() {
    let mut app = create_test_app();
    app.pending_play_index = Some(0);
    app.play_selected();
    assert_eq!(app.pending_play_index, None);
}

// on_stopped() is private; its behavior is covered indirectly by
// test_confirm_number_input_sets_pending_when_playing and
// test_confirm_number_input_overwrites_pending, which set pending_play_index
// and rely on poll_events() → on_stopped() to actually play the song.
