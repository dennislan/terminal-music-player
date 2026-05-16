use crate::playlist::Song;
use crate::stream;
use rodio::Decoder;
use rodio::stream::DeviceSinkBuilder;
use std::io::{Read, Seek};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Combined trait for Read + Seek + Send + Sync so it can be used as a trait object.
/// rodio::Decoder requires R: Read + Seek + Send + Sync + 'static.
pub trait ReadSeek: Read + Seek + Send + Sync {}
impl<T: Read + Seek + Send + Sync> ReadSeek for T {}

pub enum PlayerCommand {
    Play(Song),
    Stop,
    PauseResume,
    SeekTo(f64),
}

#[derive(Debug, Clone)]
pub enum PlayerEvent {
    Buffering(String),
    Started(String, f64),
    Position(Duration),
    Paused,
    Resumed,
    Finished,
    Stopped,
    Error(String),
}

/// Abstracts how audio is fetched for a URL.
/// Real adapter: yt-dlp. Test adapter: in-memory buffer.
pub trait AudioFetcher: Send {
    fn fetch(&self, url: &str) -> anyhow::Result<FetchedStream>;
}

/// Result of fetching audio: a readable stream + metadata.
pub struct FetchedStream {
    pub reader: Box<dyn ReadSeek + Send>,
    pub duration_secs: f64,
    /// When Some, the player thread polls this for async download errors.
    pub download_error: Option<Arc<Mutex<Option<String>>>>,
}

/// Default fetcher that uses yt-dlp via Bilibili.
pub struct YtDlpFetcher;

impl AudioFetcher for YtDlpFetcher {
    fn fetch(&self, url: &str) -> anyhow::Result<FetchedStream> {
        let stream = stream::create_audio_stream(url)?;
        Ok(FetchedStream {
            reader: Box::new(stream.handle),
            duration_secs: stream.duration_secs,
            download_error: Some(stream.download_error),
        })
    }
}

pub struct Player {
    cmd_tx: Sender<PlayerCommand>,
    event_rx: Receiver<PlayerEvent>,
}

impl Player {
    pub fn new() -> Self {
        Self::with_fetcher(Box::new(YtDlpFetcher))
    }

    pub fn with_fetcher(fetcher: Box<dyn AudioFetcher>) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();

        thread::spawn(move || {
            player_main(cmd_rx, event_tx, fetcher);
        });

        Player { cmd_tx, event_rx }
    }

    pub fn send(&self, cmd: PlayerCommand) {
        self.cmd_tx.send(cmd).ok();
    }

    pub fn try_recv(&self) -> Option<PlayerEvent> {
        self.event_rx.try_recv().ok()
    }
}

fn player_main(
    cmd_rx: Receiver<PlayerCommand>,
    event_tx: Sender<PlayerEvent>,
    fetcher: Box<dyn AudioFetcher>,
) {
    let (_sink, player) = match DeviceSinkBuilder::open_default_sink() {
        Ok(s) => {
            let mixer = s.mixer();
            let p = rodio::Player::connect_new(&mixer);
            (Some(s), Some(p))
        }
        Err(e) => {
            event_tx
                .send(PlayerEvent::Error(format!(
                    "Audio output unavailable: {}",
                    e
                )))
                .ok();
            (None, None)
        }
    };

    let mut is_playing = false;
    let mut download_error: Option<Arc<Mutex<Option<String>>>> = None;

    loop {
        if let Ok(cmd) = cmd_rx.recv_timeout(Duration::from_millis(500)) {
            match cmd {
                PlayerCommand::Play(song) => {
                    let alias = song.alias.clone();

                    event_tx.send(PlayerEvent::Buffering(alias.clone())).ok();

                    match fetcher.fetch(&song.url) {
                        Ok(stream) => {
                            let FetchedStream {
                                reader,
                                duration_secs,
                                download_error: dl_err,
                            } = stream;

                            match Decoder::new(reader) {
                                Ok(source) => {
                                    if let Some(ref p) = player {
                                        p.clear();
                                        p.append(source);
                                        p.play();
                                        is_playing = true;

                                        if song.last_position > 0.0 {
                                            let pos = Duration::from_secs_f64(
                                                song.last_position.min(duration_secs),
                                            );
                                            let _ = p.try_seek(pos);
                                        }

                                        if let Some(ref err_cell) = dl_err {
                                            if let Some(err) = err_cell.lock().unwrap().take() {
                                                p.stop();
                                                is_playing = false;
                                                event_tx.send(PlayerEvent::Error(err)).ok();
                                            } else {
                                                download_error = dl_err;
                                                event_tx
                                                    .send(PlayerEvent::Started(
                                                        alias,
                                                        duration_secs,
                                                    ))
                                                    .ok();
                                            }
                                        } else {
                                            download_error = None;
                                            event_tx
                                                .send(PlayerEvent::Started(alias, duration_secs))
                                                .ok();
                                        }
                                    }
                                }
                                Err(e) => {
                                    let err_msg = dl_err
                                        .as_ref()
                                        .and_then(|d| d.lock().unwrap().take())
                                        .unwrap_or_else(|| format!("Audio decode error: {}", e));
                                    event_tx.send(PlayerEvent::Error(err_msg)).ok();
                                }
                            }
                        }
                        Err(e) => {
                            event_tx
                                .send(PlayerEvent::Error(format!("Stream error: {}", e)))
                                .ok();
                        }
                    }
                }
                PlayerCommand::Stop => {
                    if let Some(ref p) = player {
                        p.stop();
                    }
                    is_playing = false;
                    download_error = None;
                    let _ = event_tx.send(PlayerEvent::Stopped);
                }
                PlayerCommand::PauseResume => {
                    if let Some(ref p) = player {
                        if p.is_paused() {
                            p.play();
                            event_tx.send(PlayerEvent::Resumed).ok();
                        } else if is_playing {
                            p.pause();
                            event_tx.send(PlayerEvent::Paused).ok();
                        }
                    }
                }
                PlayerCommand::SeekTo(pos_secs) => {
                    if let Some(ref p) = player {
                        let pos = Duration::from_secs_f64(pos_secs);
                        let _ = p.try_seek(pos);
                    }
                }
            }
        }

        if let Some(ref dl_err) = download_error {
            let err = dl_err.lock().unwrap().take();
            if let Some(msg) = err {
                download_error = None;
                if let Some(ref p) = player {
                    p.stop();
                }
                is_playing = false;
                event_tx.send(PlayerEvent::Error(msg)).ok();
            }
        }

        if let Some(ref p) = player {
            if is_playing {
                let pos = p.get_pos();
                event_tx.send(PlayerEvent::Position(pos)).ok();
            }

            if is_playing && p.empty() && !p.is_paused() {
                is_playing = false;
                event_tx.send(PlayerEvent::Finished).ok();
            }
        }
    }
}
