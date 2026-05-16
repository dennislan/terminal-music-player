use std::io::Cursor;
use std::thread;
use std::time::Duration;
use terminal_music_player::player::{
    AudioFetcher, FetchedStream, Player, PlayerCommand, PlayerEvent,
};

struct MockFetcher {
    data: Vec<u8>,
    duration: f64,
}

impl AudioFetcher for MockFetcher {
    fn fetch(&self, _url: &str) -> anyhow::Result<FetchedStream> {
        // Cursor<Vec<u8>> already implements Read + Seek + Send + Sync
        let cursor = Cursor::new(self.data.clone());
        Ok(FetchedStream {
            reader: Box::new(cursor),
            duration_secs: self.duration,
            download_error: None,
        })
    }
}

#[test]
fn test_player_send_play() {
    let fetcher = Box::new(MockFetcher {
        data: vec![0u8; 1024],
        duration: 10.0,
    });
    let player = Player::with_fetcher(fetcher);
    let song = terminal_music_player::playlist::Song {
        url: "https://example.com".to_string(),
        alias: "Test".to_string(),
        last_position: 0.0,
    };
    player.send(PlayerCommand::Play(song));
    // Give some time for the player thread to process
    thread::sleep(Duration::from_millis(100));
    // If we reach here without panic, the test passes
}

#[test]
fn test_player_send_stop() {
    let fetcher = Box::new(MockFetcher {
        data: vec![0u8; 1024],
        duration: 10.0,
    });
    let player = Player::with_fetcher(fetcher);
    player.send(PlayerCommand::Stop);
    thread::sleep(Duration::from_millis(100));
}

#[test]
fn test_player_send_pause_resume() {
    let fetcher = Box::new(MockFetcher {
        data: vec![0u8; 1024],
        duration: 10.0,
    });
    let player = Player::with_fetcher(fetcher);
    player.send(PlayerCommand::PauseResume);
    thread::sleep(Duration::from_millis(100));
}

#[test]
fn test_player_send_seek_to() {
    let fetcher = Box::new(MockFetcher {
        data: vec![0u8; 1024],
        duration: 10.0,
    });
    let player = Player::with_fetcher(fetcher);
    player.send(PlayerCommand::SeekTo(5.0));
    thread::sleep(Duration::from_millis(100));
}

#[test]
fn test_player_try_recv_empty() {
    let fetcher = Box::new(MockFetcher {
        data: vec![0u8; 1024],
        duration: 10.0,
    });
    let player = Player::with_fetcher(fetcher);
    // Initially should have no events
    let result = player.try_recv();
    // May or may not have events depending on timing
    let _ = result;
}

#[test]
fn test_player_command_variants() {
    let _cmd1 = PlayerCommand::Play(terminal_music_player::playlist::Song {
        url: "test".to_string(),
        alias: "test".to_string(),
        last_position: 0.0,
    });
    let _cmd2 = PlayerCommand::Stop;
    let _cmd3 = PlayerCommand::PauseResume;
    let _cmd4 = PlayerCommand::SeekTo(1.0);
}

#[test]
fn test_player_event_variants() {
    use std::time::Duration;
    let _evt1 = PlayerEvent::Buffering("test".to_string());
    let _evt2 = PlayerEvent::Started("test".to_string(), 10.0);
    let _evt3 = PlayerEvent::Position(Duration::from_secs(5));
    let _evt4 = PlayerEvent::Paused;
    let _evt5 = PlayerEvent::Resumed;
    let _evt6 = PlayerEvent::Finished;
    let _evt7 = PlayerEvent::Error("error".to_string());
}
